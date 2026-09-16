import { colorFormats } from "./color";

const pad = (n: number) => String(n).padStart(2, "0");

/** 列表里用的短时间：今天只显示 HH:MM，昨天及更早显示 M/D。 */
export function formatListTime(epochMs: number): string {
  const d = new Date(epochMs);
  const now = new Date();

  const sameDay =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate();

  if (sameDay) {
    return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }
  return `${d.getMonth() + 1}/${d.getDate()}`;
}

/** 预览区用的完整时间。 */
export function formatFullTime(epochMs: number): string {
  const d = new Date(epochMs);
  return (
    `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
    `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  );
}

/** 摘要：压掉换行与连续空白，避免列表行被撑成多行。 */
export function summarize(text: string | null, max = 200): string {
  if (!text) return "";
  const flat = text.replace(/\s+/g, " ").trim();
  return flat.length > max ? `${flat.slice(0, max)}…` : flat;
}

/** 文件名列表 → 展示用的 chip 数据。 */
export function parseFileList(content: string | null): string[] {
  if (!content) return [];
  const trimmed = content.trim();
  if (trimmed.startsWith("[")) {
    try {
      const parsed: unknown = JSON.parse(trimmed);
      if (Array.isArray(parsed)) {
        return parsed.filter((v): v is string => typeof v === "string");
      }
    } catch {
      // 不是 JSON 就按分隔符切
    }
  }
  return trimmed.split(/\r?\n|\s*\|\s*/).filter(Boolean);
}

/** 从完整路径里取出文件名。 */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

/** Only absolute web/mail links are exported. Invalid input is reported by the copy UI. */
export function markdownLink(rawUrl: string, title?: string | null): string {
  const raw = rawUrl.trim();
  if (!raw || /[\u0000-\u001f\u007f]/.test(raw)) throw new Error("Invalid URL");
  const url = new URL(raw);
  if (!["http:", "https:", "mailto:"].includes(url.protocol)) throw new Error("Unsupported URL scheme");
  const label = (title?.trim() || url.host || raw).replace(/\s+/g, " ")
    .replace(/[\`*_[\]{}()<>!#|~&]/g, "\$&");
  // Angle destination plus percent encoding prevents closing the destination or injecting HTML.
  const destination = url.href.replace(/[<>\()'"`\s]/g, (c) => encodeURIComponent(c).replace(/[()']/g, (v) => `%${v.charCodeAt(0).toString(16).toUpperCase()}`));
  return `[${label}](<${destination}>)`;
}

/** Validated canonical sRGB value, shared by list and preview swatches. */
export function cssColor(value: string): string | null {
  return colorFormats(value)[1]?.[1] ?? null;
}
