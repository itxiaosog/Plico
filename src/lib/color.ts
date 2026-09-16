type Color = [number, number, number, number];
const number = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?$/i;
const clamp = (x: number, max = 1) => Math.min(max, Math.max(0, x));
const rounded = (x: number) => Number(x.toFixed(4));
function scalar(raw: string, max: number): number | null {
  const percent = raw.endsWith("%");
  const text = percent ? raw.slice(0, -1) : raw;
  if (!number.test(text) || !Number.isFinite(Number(text))) return null;
  return clamp(Number(text) * (percent ? max / 100 : 1), max);
}

/** Bounded CSS sRGB parser. Unsupported syntax returns null, never a partial match. */
export function parseColor(raw: string): Color | null {
  const text = raw.trim();
  if (/^#(?:[\da-f]{3}|[\da-f]{4}|[\da-f]{6}|[\da-f]{8})$/i.test(text)) {
    let hex = text.slice(1);
    if (hex.length < 5) hex = [...hex].map((c) => c + c).join("");
    return [0, 2, 4, 6].map((i) => i === 6 ? (hex.length === 8 ? parseInt(hex.slice(i), 16) / 255 : 1) : parseInt(hex.slice(i, i + 2), 16)) as Color;
  }
  const match = /^(rgba?|hsla?)\(([^()]*)\)$/i.exec(text);
  if (!match) return null;
  const body = match[2]!.trim();
  let channels: string[], alpha: string | undefined;
  if (body.includes(",")) {
    if (body.includes("/")) return null;
    const parts = body.split(",").map((v) => v.trim());
    if (parts.length !== 3 && parts.length !== 4) return null;
    channels = parts.slice(0, 3); alpha = parts[3];
  } else {
    const parts = body.split("/");
    if (parts.length > 2) return null;
    channels = parts[0]!.trim().split(/\s+/); alpha = parts[1]?.trim();
  }
  if (channels.length !== 3) return null;
  const a = alpha === undefined ? 1 : scalar(alpha, 1);
  if (a === null) return null;
  if (match[1]!.toLowerCase().startsWith("rgb")) {
    if (body.includes(",") && !channels.every((v) => v.endsWith("%") === channels[0]!.endsWith("%"))) return null;
    const rgb = channels.map((v) => scalar(v, 255));
    return rgb.some((v) => v === null) ? null : [...rgb, a] as Color;
  }
  const hue = /^(.+?)(deg|grad|rad|turn)?$/i.exec(channels[0]!);
  if (!hue || !number.test(hue[1]!) || !Number.isFinite(Number(hue[1]!))) return null;
  if (!channels[1]!.endsWith("%") || !channels[2]!.endsWith("%")) return null;
  const s = scalar(channels[1]!, 1), l = scalar(channels[2]!, 1);
  if (s === null || l === null) return null;
  const factor = { deg: 1, grad: 0.9, rad: 180 / Math.PI, turn: 360 }[hue[2]?.toLowerCase() ?? "deg"]!;
  const degrees = Number(hue[1]!) * factor;
  if (!Number.isFinite(degrees)) return null;
  const h = ((degrees % 360) + 360) % 360 / 30;
  const f = (n: number) => { const k = (n + h) % 12; return 255 * (l - s * Math.min(l, 1 - l) * Math.max(-1, Math.min(k - 3, 9 - k, 1))); };
  return [f(0), f(8), f(4), a];
}

export function colorFormats(raw: string): Array<[string, string]> {
  const color = parseColor(raw);
  if (!color) return [];
  const [r, g, b, a] = color;
  const rgb = [r, g, b].map(rounded);
  const hex = `#${[r, g, b, ...(a < 1 ? [a * 255] : [])].map((v) => Math.round(v).toString(16).padStart(2, "0")).join("").toUpperCase()}`;
  const max = Math.max(r, g, b), min = Math.min(r, g, b), d = max - min;
  const l = (max + min) / 510;
  const s = d === 0 ? 0 : d / 255 / (1 - Math.abs(2 * l - 1));
  let h = d === 0 ? 0 : max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  h = ((h * 60) + 360) % 360;
  return [["HEX", hex], ["RGB", a < 1 ? `rgba(${rgb.join(", ")}, ${rounded(a)})` : `rgb(${rgb.join(", ")})`], ["HSL", `${a < 1 ? "hsla" : "hsl"}(${rounded(h)}, ${rounded(s * 100)}%, ${rounded(l * 100)}%${a < 1 ? `, ${rounded(a)}` : ""})`]];
}
