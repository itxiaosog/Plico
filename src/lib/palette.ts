/**
 * F16 分组色板（《前端视觉规范.md》第 7 节）。
 *
 * 存的是**CSS 变量名**而不是色值本身。理由：同一份数据要在浅色和深色两套主题下
 * 各显示一次（浅色 `#378ADD` / 深色 `#85B7EB`），存变量名才能真正跟随主题，
 * 存色值就得在库里存两份、还得在切主题时同步。
 *
 * 值直接写 `var(--pl-group-*)` 字符串（而不是套一层 `var()`），是为了让
 * `style={{ background: group.color }}` 这种最朴素的用法就能生效。
 */

import type { MsgKey } from "./i18n";

export interface PaletteEntry {
  /** 存进 `groups.color` 的值。 */
  value: string;
  /** 色板里那个圆点的背景色，同样是 CSS 变量。 */
  swatch: string;
  /** i18n key，色板工具提示用。用 `MsgKey` 而不是 `string`，
   *  否则 `t(entry.labelKey)` 过不了类型检查 —— 文案表是键字面量约束的。 */
  labelKey: MsgKey;
}

/**
 * 8 色，顺序与规范第 7 节表格一致。
 *
 * 「石墨」排在最后并且是**默认色**（用户不选时用它）：它是唯一一个不带彩度的，
 * 放在末尾不会打乱前 7 个彩色的扫视节奏。
 */
export const GROUP_PALETTE: PaletteEntry[] = [
  { value: "var(--pl-group-mint)", swatch: "var(--pl-group-mint)", labelKey: "colorMint" },
  { value: "var(--pl-group-sky)", swatch: "var(--pl-group-sky)", labelKey: "colorSky" },
  { value: "var(--pl-group-indigo)", swatch: "var(--pl-group-indigo)", labelKey: "colorIndigo" },
  { value: "var(--pl-group-lotus)", swatch: "var(--pl-group-lotus)", labelKey: "colorLotus" },
  { value: "var(--pl-group-clay)", swatch: "var(--pl-group-clay)", labelKey: "colorClay" },
  { value: "var(--pl-group-amber)", swatch: "var(--pl-group-amber)", labelKey: "colorAmber" },
  { value: "var(--pl-group-moss)", swatch: "var(--pl-group-moss)", labelKey: "colorMoss" },
  { value: "var(--pl-group-graphite)", swatch: "var(--pl-group-graphite)", labelKey: "colorGraphite" },
];

/** 没选颜色时的默认值 —— 石墨（规范第 7 节）。 */
export const DEFAULT_GROUP_COLOR = "var(--pl-group-graphite)";

/**
 * 分组圆点的显示色。
 *
 * 老库里可能存在裸十六进制（mock 的种子数据早期就写过 `#12A594`），
 * 或者是 `null`。两者都原样放行给 CSS —— 浏览器两种都认。
 */
export function groupDotColor(color: string | null | undefined): string {
  return color && color.trim().length > 0 ? color : DEFAULT_GROUP_COLOR;
}

/** 判断某个色值是否就是当前选中的那个（含「null 等于默认色」的等价）。 */
export function isColorActive(current: string | null | undefined, candidate: string): boolean {
  return groupDotColor(current) === candidate;
}
