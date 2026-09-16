import { convertFileSrc, invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";

import { mockInvoke, mockListen } from "./mock";

export const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    return tauriInvoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

export function listen(event: string, handler: () => void): Promise<UnlistenFn> {
  if (isTauri) {
    return tauriListen(event, handler);
  }
  return mockListen(event, handler);
}

export function imageUrl(path: string | null): string | null {
  if (!path) return null;
  if (isTauri) {
    return convertFileSrc(path);
  }
  return path.startsWith("data:") ? path : null;
}
