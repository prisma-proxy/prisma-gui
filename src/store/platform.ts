import { create } from "zustand";

export type PlatformName = "windows" | "macos" | "linux" | "ios" | "android" | "unknown";

interface PlatformStore {
  platform: PlatformName;
  isMobile: boolean;
  isDesktop: boolean;
  ready: boolean;
  init: (p: PlatformName) => void;
}

/** Detect platform early using user agent heuristics (synchronous). */
function detectPlatformSync(): PlatformName {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("android")) return "android";
  if (ua.includes("iphone") || ua.includes("ipad")) return "ios";
  if (ua.includes("windows")) return "windows";
  if (ua.includes("mac")) return "macos";
  if (ua.includes("linux")) return "linux";
  return "unknown";
}

const initialPlatform = detectPlatformSync();

export const usePlatformStore = create<PlatformStore>()((set) => ({
  platform: initialPlatform,
  isMobile: initialPlatform === "ios" || initialPlatform === "android",
  isDesktop: initialPlatform !== "ios" && initialPlatform !== "android" && initialPlatform !== "unknown",
  ready: initialPlatform !== "unknown",
  init: (p) =>
    set({
      platform: p,
      isMobile: p === "ios" || p === "android",
      isDesktop: p !== "ios" && p !== "android" && p !== "unknown",
      ready: true,
    }),
}));

/** Synchronous platform check for use in callbacks and non-React code. */
export function isMobileSync(): boolean {
  return usePlatformStore.getState().isMobile;
}

export function isDesktopSync(): boolean {
  return usePlatformStore.getState().isDesktop;
}
