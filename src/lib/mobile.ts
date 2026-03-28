import { useSettings } from "@/store/settings";
import { isMobileSync } from "@/store/platform";
import { api } from "@/lib/commands";
import { MODE_SOCKS5, MODE_TUN } from "@/lib/types";

/** Derive the effective proxy modes based on platform and connection mode. */
export function getEffectiveModes(): number {
  const mobile = isMobileSync();
  if (mobile) {
    const settings = useSettings.getState();
    const modes = settings.connectionMode === "vpn"
      ? MODE_SOCKS5 | MODE_TUN
      : MODE_SOCKS5;
    console.log(`[getEffectiveModes] mobile=true, connectionMode=${settings.connectionMode}, modes=0x${modes.toString(16)}`);
    return modes;
  }
  const modes = useSettings.getState().proxyModes;
  console.log(`[getEffectiveModes] mobile=false, proxyModes=0x${modes.toString(16)}`);
  return modes;
}

/** Start mobile VPN service. The Rust side polls for the TUN fd asynchronously. */
export async function startMobileVpnIfNeeded(modes: number): Promise<void> {
  if (!isMobileSync() || (modes & MODE_TUN) === 0) return;
  await api.startVpnService();
  // No sleep needed — the Rust start_vpn_service command spawns a background
  // thread that polls the Kotlin VPN service for the TUN fd.
}

/** Stop mobile VPN service if running in VPN mode. No-op on desktop. */
export function stopMobileVpnIfNeeded(): void {
  if (!isMobileSync()) return;
  const settings = useSettings.getState();
  if (settings.connectionMode === "vpn") {
    api.stopVpnService().catch(() => {});
  }
}
