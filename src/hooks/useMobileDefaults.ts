import { useEffect } from "react";
import { usePlatformStore } from "@/store/platform";
import { useSettings } from "@/store/settings";
import { MODE_SOCKS5, MODE_SYSTEM_PROXY } from "@/lib/types";

/**
 * One-time migration: fix proxyModes default on mobile.
 *
 * Desktop defaults to MODE_SYSTEM_PROXY (0x02) which is meaningless on mobile.
 * On mobile, set to MODE_SOCKS5 (0x01) if the user hasn't customized it.
 */
export function useMobileDefaults() {
  const ready = usePlatformStore((s) => s.ready);
  const isMobile = usePlatformStore((s) => s.isMobile);

  useEffect(() => {
    if (!ready || !isMobile) return;
    const settings = useSettings.getState();
    // Only fix if still at the desktop default (MODE_SYSTEM_PROXY)
    if (settings.proxyModes === MODE_SYSTEM_PROXY) {
      settings.setProxyModes(MODE_SOCKS5);
    }
  }, [ready, isMobile]);
}
