/**
 * Tauri 调用桥。
 *
 * 在 Tauri 里走真实 IPC；在普通浏览器里（`npm run dev` 直接打开）
 * 自动降级到 mock 层，这样面板 UI 可以脱离 Rust 后端独立开发和验收。
 */
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

/**
 * 本地文件路径 → 可渲染的 URL。
 *
 * Tauri 下走 asset 协议（scope 放行 `$RESOURCE/data/images` 与 `thumbs`，
 * 外加 `$APPDATA` 下同名两个目录兜底，见 tauri.conf.json）；
 * 浏览器 mock 环境里条目本身存的就是 data URI，直接透传即可。
 */
export function imageUrl(path: string | null): string | null {
  if (!path) return null;
  if (isTauri) {
    return convertFileSrc(path);
  }
  return path.startsWith("data:") ? path : null;
}
