import type { MsgKey } from "./i18n";

export interface PaletteEntry {
  value: string;
  swatch: string;
  labelKey: MsgKey;
}

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

export const DEFAULT_GROUP_COLOR = "var(--pl-group-graphite)";

export function groupDotColor(color: string | null | undefined): string {
  return color && color.trim().length > 0 ? color : DEFAULT_GROUP_COLOR;
}

export function isColorActive(current: string | null | undefined, candidate: string): boolean {
  return groupDotColor(current) === candidate;
}
