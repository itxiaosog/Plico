import type { ReactNode } from "react";

import type { ItemType } from "../types";

/**
 * 类型图标：像素填充风。用 fill 而非 stroke，方块化无圆角。
 * 颜色由徽标的 data-type CSS 选择器注入（currentColor 继承）。
 * 14×14 网格，每个像素 = 1 单位。
 */
const PIXEL_ICONS: Record<ItemType, ReactNode> = {
  // 文本：三行横线（像素风，每行 2px 高）
  text: (
    <>
      <rect x="2" y="3" width="8" height="1.5" />
      <rect x="2" y="6.25" width="8" height="1.5" />
      <rect x="2" y="9.5" width="5" height="1.5" />
    </>
  ),
  // 富文本：文本行 + 一个加粗标记
  rich_text: (
    <>
      <rect x="2" y="3" width="8" height="1.5" />
      <rect x="2" y="6.25" width="5" height="1.5" />
      <rect x="8" y="6.25" width="2" height="1.5" />
      <rect x="2" y="9.5" width="6" height="1.5" />
    </>
  ),
  // 图片：方块框 + 山景 + 太阳
  image: (
    <>
      <rect x="2" y="3" width="10" height="8" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <rect x="4" y="5" width="2" height="2" />
      <path d="M3 10 L5.5 7 L7 8.5 L9 6 L11 10 Z" />
    </>
  ),
  // 文件：文件夹形
  files: (
    <>
      <path d="M2 4 L2 11 L12 11 L12 5 L7 5 L6 4 Z" />
    </>
  ),
  // 链接：两个方块用线连
  link: (
    <>
      <rect x="2" y="2" width="4" height="4" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <rect x="8" y="8" width="4" height="4" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <path d="M6 4 L8 4 L8 10 L6 10" fill="none" stroke="currentColor" strokeWidth="1.5" />
    </>
  ),
  // 颜色：水滴/调色板
  color: (
    <>
      <path d="M7 2 C7 2 3 6 3 9 C3 11 5 12 7 12 C9 12 11 11 11 9 C11 6 7 2 7 2 Z" />
    </>
  ),
};

interface Props {
  type: ItemType;
  size?: number;
}

export function TypeIcon({ type, size = 14 }: Props) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 14 14"
      fill="currentColor"
      aria-hidden="true"
    >
      {PIXEL_ICONS[type]}
    </svg>
  );
}

/** 图钉。列表里唯一的彩色元素。 */
export function PinIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="4" y="1" width="4" height="1.5" />
      <rect x="5.25" y="2.5" width="1.5" height="3.5" />
      <rect x="3.5" y="6" width="5" height="2" />
      <rect x="5.25" y="8" width="1.5" height="3" />
    </svg>
  );
}

export function SearchIcon({ size = 14 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 14 14"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="square"
      aria-hidden="true"
    >
      <circle cx="6" cy="6" r="3.5" />
      <path d="M9 9 L12 12" />
    </svg>
  );
}

export function TrashIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="1.5" y="2.5" width="9" height="1.5" />
      <rect x="4" y="1" width="4" height="1.5" />
      <rect x="3" y="4" width="1.5" height="7" />
      <rect x="5.25" y="4" width="1.5" height="7" />
      <rect x="7.5" y="4" width="1.5" height="7" />
    </svg>
  );
}

/**
 * 改名（F16 分组行）。笔杆用阶梯方块拼出对角线，笔尖收窄一格。
 *
 * **不要退回用 `✎` 这种文字字形当图标。** 它在 `PS2P` 的 unicode-range
 * （U+0000-00FF / U+2000-206F / U+3000-303F）之外，`Fusion Pixel` 里也没有，
 * 于是落到系统字体（雅黑）上渲染 —— 笔画粗细、视觉大小都和旁边的像素图标
 * 对不上（实测 `✎` 占 16px 宽，`×` 只占 10px）。字体回退不可控，图标必须是图形。
 */
export function PencilIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="8.5" y="1" width="2.5" height="2.5" />
      <rect x="6.5" y="3" width="2.5" height="2.5" />
      <rect x="4.5" y="5" width="2.5" height="2.5" />
      <rect x="2.5" y="7" width="2.5" height="2.5" />
      <rect x="1" y="9" width="1.5" height="1.5" />
    </svg>
  );
}

/** 设置入口。用「滑杆」而不是齿轮：12px 下的齿轮齿形会糊成一团。 */
export function TuneIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="1" y="2.5" width="7" height="1.5" />
      <rect x="1" y="8" width="7" height="1.5" />
      <rect x="3.5" y="1.5" width="2" height="3.5" />
      <rect x="6.5" y="7" width="2" height="3.5" />
    </svg>
  );
}

/**
 * F16 批量选择的勾选标记。替换类型图标出现在同一个徽标位里 ——
 * 用替换而不是在旁边再加一个方块，是为了不让行高和左侧留白发生变化。
 */
export function CheckIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="square"
      strokeLinejoin="miter"
      aria-hidden="true"
    >
      <path d="M2 6 L5 9 L10 3" />
    </svg>
  );
}

/**
 * 折叠预览区用的方向标（F5）。
 * `dir="right"` 表示「向右侧收起」，`dir="left"` 表示「向左展开」。
 */
export function ChevronIcon({ dir, size = 12 }: { dir: "left" | "right"; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="square"
      strokeLinejoin="miter"
      aria-hidden="true"
    >
      <path d={dir === "right" ? "M4 2 L8 6 L4 10" : "M8 2 L4 6 L8 10"} />
    </svg>
  );
}
