import { useEffect, useState } from "react";
import { usePlatformStore } from "@/store/platform";
import type { PlatformName } from "@/store/platform";

export function usePlatform() {
  const [platform, setPlatform] = useState<PlatformName>("unknown");
  const initStore = usePlatformStore((s) => s.init);

  useEffect(() => {
    // Dynamically import to avoid SSR issues and handle missing Tauri runtime
    import("@tauri-apps/plugin-os")
      .then(({ platform: getPlatform }) => getPlatform())
      .then((p) => {
        const name = p as PlatformName;
        setPlatform(name);
        initStore(name);
      })
      .catch(() => {
        // Fallback: detect mobile by screen width when running in browser
        const w = window.innerWidth;
        const name: PlatformName = w < 600 ? "android" : "windows";
        setPlatform(name);
        initStore(name);
      });
  }, [initStore]);

  const isMobile  = platform === "ios" || platform === "android";
  const isDesktop = !isMobile && platform !== "unknown";

  return { platform, isMobile, isDesktop };
}
