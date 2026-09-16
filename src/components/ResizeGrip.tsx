import { useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauri } from "../lib/bridge";

/** 与 Rust 侧 `PANEL_W_RANGE` / `PANEL_H_RANGE` 对齐。 */
const MIN_W = 320;
const MIN_H = 200;
/** 展开预览区时窗口的最小宽度：列表 320 + 预览 240 + 分隔线 1。 */
const MIN_W_WITH_PREVIEW = 561;

interface Props {
  /** 预览区是否展开，决定最小宽度。 */
  previewExpanded: boolean;
  /** 拖完之后落库（浏览器预览用；真机由后端在隐藏时统一记忆）。 */
  onResized(width: number, height: number): void;
}

/**
 * 右下角的缩放握把（F5）。
 *
 * 无边框窗口在 Windows 上仍然有不可见的缩放边框，所以「拖边缘」其实本来就能用；
 * 这个握把解决的是**可发现性** —— 一条 0.5px 的边框没人知道可以拖。
 *
 * 真机走 Tauri 的 `startResizeDragging`：把调整尺寸的活交给系统，
 * 拖拽过程中的跟手程度、多屏 DPI、贴边吸附都是系统实现，比自己算 mousemove 好。
 * 浏览器预览里没有窗口可调，退化成直接改 `#root` 的尺寸，让这个交互能验。
 */
export function ResizeGrip({ previewExpanded, onResized }: Props) {
  const drag = useRef<{ x: number; y: number; w: number; h: number } | null>(null);

  const onMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    if (isTauri) {
      void getCurrentWindow().startResizeDragging("SouthEast");
      return;
    }

    const root = document.getElementById("root");
    if (!root) return;
    drag.current = { x: e.clientX, y: e.clientY, w: root.offsetWidth, h: root.offsetHeight };

    const minW = previewExpanded ? MIN_W_WITH_PREVIEW : MIN_W;
    const onMove = (ev: MouseEvent) => {
      const d = drag.current;
      if (!d) return;
      root.style.width = `${Math.max(minW, d.w + ev.clientX - d.x)}px`;
      root.style.height = `${Math.max(MIN_H, d.h + ev.clientY - d.y)}px`;
    };
    const onUp = () => {
      drag.current = null;
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      onResized(root.offsetWidth, root.offsetHeight);
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  return (
    <button
      type="button"
      className="pl-resize-grip"
      aria-label="调整面板大小"
      onMouseDown={onMouseDown}
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <path
          d="M9 4.5L4.5 9M9 7.5L7.5 9"
          stroke="currentColor"
          strokeWidth="1.1"
          strokeLinecap="round"
          fill="none"
        />
      </svg>
    </button>
  );
}
