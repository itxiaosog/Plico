const PALETTE = ["mint", "sky", "indigo", "lotus", "clay", "amber", "moss", "graphite"] as const;

export interface SiteBadge {
  letter: string;
  color: string;
}

function fnv1a(input: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < input.length; i += 1) {
    h ^= input.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h >>> 0;
}

export function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url.trim();
  }
}

export function siteBadge(url: string): SiteBadge {
  const host = hostOf(url).toLowerCase();
  const bare = host.replace(/^www\./, "");
  const letter =
    [...bare].find((c) => /[a-z0-9\u4e00-\u9fa5]/i.test(c))?.toUpperCase() ?? "?";
  const color = `var(--pl-group-${PALETTE[fnv1a(bare) % PALETTE.length]})`;
  return { letter, color };
}
