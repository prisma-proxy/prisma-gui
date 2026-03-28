import { useCallback } from "react";
import { useStore } from "@/store";
import { notify } from "@/store/notifications";
import { api } from "@/lib/commands";
import { useRules } from "@/store/rules";
import { useRuleProviders } from "@/store/ruleProviders";
import { useSettings } from "@/store/settings";
import { isMobileSync } from "@/store/platform";
import { mergeSettingsIntoConfig } from "@/lib/buildConfig";
import { getEffectiveModes, startMobileVpnIfNeeded, stopMobileVpnIfNeeded } from "@/lib/mobile";
import type { Profile } from "@/lib/types";
import { MODE_SOCKS5, MODE_SYSTEM_PROXY, MODE_TUN } from "@/lib/types";

export function useConnection() {
  const setActiveProfileIdx = useStore((s) => s.setActiveProfileIdx);
  const setManualDisconnect = useStore((s) => s.setManualDisconnect);
  const setConnectStartTime = useStore((s) => s.setConnectStartTime);
  const setConnected = useStore((s) => s.setConnected);
  const setProxyModes = useSettings((s) => s.setProxyModes);

  const connectTo = useCallback(async (profile: Profile, modes: number): Promise<boolean> => {
    console.log(`[connect] modes=0x${modes.toString(16)}, isMobile=${isMobileSync()}, connectionMode=${useSettings.getState().connectionMode}`);
    // TUN mode requires platform-specific preparation
    if ((modes & MODE_TUN) !== 0) {
      console.log("[connect] TUN mode requested, starting VPN flow...");
      if (isMobileSync()) {
        // Mobile VPN flow: check permission → start VPN service → connect
        try {
          console.log("[connect] checking VPN permission...");
          const hasPermission = await api.checkVpnPermission();
          console.log(`[connect] VPN permission: ${hasPermission}`);
          if (!hasPermission) {
            const granted = await api.requestVpnPermission();
            console.log(`[connect] VPN permission requested: ${granted}`);
            if (!granted) {
              notify.warning("VPN permission is required. Please grant it in system settings.");
              return false;
            }
          }
          console.log("[connect] starting VPN service...");
          await startMobileVpnIfNeeded(modes);
          console.log("[connect] VPN service started");
        } catch (e) {
          console.error("[connect] VPN error:", e);
          notify.error(`VPN service error: ${String(e)}`);
          return false;
        }
      } else {
        // Desktop: TUN mode requires admin privileges
        try {
          const elevated = await api.checkElevation();
          if (!elevated) {
            notify.warning("TUN mode requires administrator privileges. Please restart the app as administrator.");
            return false;
          }
        } catch { /* checkElevation not available on this platform */ }
      }
    }

    const profiles = useStore.getState().profiles;
    const idx = profiles.findIndex((p) => p.id === profile.id);
    if (idx >= 0) setActiveProfileIdx(idx);
    setConnectStartTime(Date.now());
    try {
      const enabledProviders = useRuleProviders.getState().providers
        .filter((p) => p.enabled)
        .map((p) => ({ name: p.name, url: p.url, behavior: p.behavior, action: p.action }));
      const config = mergeSettingsIntoConfig(
        profile.config as Record<string, unknown>,
        useSettings.getState(),
        useRules.getState().rules,
        enabledProviders.length > 0 ? enabledProviders : undefined,
        modes,
      );
      const routing = config.routing as { rules?: unknown[] } | undefined;
      console.log(`[connect] ${routing?.rules?.length ?? 0} routing rules, ${enabledProviders.length} providers`);

      const tunInConfig = !!(config as Record<string, unknown>).tun;
      console.log(`[connect] calling api.connect, modes=0x${modes.toString(16)}, tun_in_config=${tunInConfig}`);

      const CONNECT_TIMEOUT = 30_000;
      await Promise.race([
        api.connect(JSON.stringify(config), modes),
        new Promise<never>((_, reject) =>
          setTimeout(() => reject(new Error("Connection timed out after 30s")), CONNECT_TIMEOUT)
        ),
      ]);
      api.setActiveProfileId(profile.id).catch(() => {});
      return true;
    } catch (e) {
      notify.error(String(e));
      setConnectStartTime(null);
      setConnected(false);
      // If we started VPN service but connect failed, stop it
      if ((modes & MODE_TUN) !== 0) stopMobileVpnIfNeeded();
      return false;
    }
  }, [setActiveProfileIdx, setConnectStartTime, setConnected]);

  const disconnect = useCallback(async () => {
    try {
      setManualDisconnect(true);
      // Stop VPN service if running on mobile in VPN mode
      stopMobileVpnIfNeeded();
      await api.disconnect();
    } catch (e) {
      notify.error(String(e));
    }
  }, [setManualDisconnect]);

  const switchTo = useCallback(async (profile: Profile, modes: number) => {
    setManualDisconnect(true);
    try { await api.disconnect(); } catch {}
    await new Promise(r => setTimeout(r, 100));
    setManualDisconnect(false);
    await connectTo(profile, modes);
  }, [connectTo, setManualDisconnect]);

  const toggle = useCallback(async () => {
    const store = useStore.getState();
    if (store.connected) {
      await disconnect();
    } else {
      const profile = store.activeProfileIdx !== null
        ? store.profiles[store.activeProfileIdx]
        : store.profiles[0];
      if (!profile) return;

      const modes = getEffectiveModes();

      await connectTo(profile, modes);
    }
  }, [connectTo, disconnect]);

  const toggleProxyOnly = useCallback(async () => {
    const store = useStore.getState();
    if (store.connected) {
      await disconnect();
    } else {
      const profile = store.activeProfileIdx !== null
        ? store.profiles[store.activeProfileIdx]
        : store.profiles[0];
      if (profile) {
        setProxyModes(MODE_SOCKS5);
        await connectTo(profile, MODE_SOCKS5);
      }
    }
  }, [connectTo, disconnect, setProxyModes]);

  const switchProxyMode = useCallback(async (oldModes: number, newModes: number) => {
    if (newModes === 0) newModes = MODE_SYSTEM_PROXY;
    const store = useStore.getState();
    if (store.connected) {
      // TUN requires a full reconnect (can't be hot-toggled)
      const hadTun = (oldModes & MODE_TUN) !== 0;
      const hasTun = (newModes & MODE_TUN) !== 0;
      if (hadTun !== hasTun) {
        setProxyModes(newModes);
        api.setTrayProxyMode(newModes).catch(() => {});
        const profile = store.activeProfileIdx !== null
          ? store.profiles[store.activeProfileIdx]
          : store.profiles[0];
        if (profile) await switchTo(profile, newModes);
        return;
      }

      // System proxy can be hot-toggled
      const hadSystem = (oldModes & MODE_SYSTEM_PROXY) !== 0;
      const hasSystem = (newModes & MODE_SYSTEM_PROXY) !== 0;
      if (hadSystem && !hasSystem) {
        api.clearSystemProxy().catch(() => {});
      } else if (!hadSystem && hasSystem) {
        const httpPort = useSettings.getState().httpPort || 0;
        if (httpPort > 0) {
          api.setSystemProxy("127.0.0.1", httpPort).catch(() => {});
        }
      }
    }
    setProxyModes(newModes);
    api.setTrayProxyMode(newModes).catch(() => {});
  }, [setProxyModes, switchTo]);

  return { connectTo, disconnect, switchTo, toggle, toggleProxyOnly, switchProxyMode };
}
