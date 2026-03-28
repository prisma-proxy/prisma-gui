import { create } from "zustand";

export type PlatformName = "windows" | "macos" | "linux" | "ios" | "android" | "unknown";

interface PlatformStore {
  platform: PlatformName;
  isMobile: boolean;
  isDesktop: boolean;
  ready: boolean;
  init: (p: PlatformName) => void;
}

export const usePlatformStore = create<PlatformStore>()((set) => ({
  platform: "unknown",
  isMobile: false,
  isDesktop: false,
  ready: false,
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
