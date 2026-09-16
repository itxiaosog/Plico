import { useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauri } from "../lib/bridge";

const MIN_W = 320;
const MIN_H = 200;
const MIN_W_WITH_PREVIEW = 561;

interface Props {
  previewExpanded: boolean;
  onResized(width: number, height: number): void;
}

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
