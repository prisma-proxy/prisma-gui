import { useSettings } from "@/store/settings";
import { isMobileSync } from "@/store/platform";
import { api } from "@/lib/commands";
import { MODE_SOCKS5, MODE_TUN } from "@/lib/types";

/** Derive the effective proxy modes based on platform and connection mode. */
export function getEffectiveModes(): number {
  if (isMobileSync()) {
    const settings = useSettings.getState();
    return settings.connectionMode === "vpn"
      ? MODE_SOCKS5 | MODE_TUN
      : MODE_SOCKS5;
  }
  return useSettings.getState().proxyModes;
}

/** Start mobile VPN service and wait for fd readiness. No-op on desktop. */
export async function startMobileVpnIfNeeded(modes: number): Promise<void> {
  if (!isMobileSync() || (modes & MODE_TUN) === 0) return;
  await api.startVpnService();
  await new Promise(r => setTimeout(r, 500));
}

/** Stop mobile VPN service if running in VPN mode. No-op on desktop. */
export function stopMobileVpnIfNeeded(): void {
  if (!isMobileSync()) return;
  const settings = useSettings.getState();
  if (settings.connectionMode === "vpn") {
    api.stopVpnService().catch(() => {});
  }
}
