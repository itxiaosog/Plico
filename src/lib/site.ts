/**
 * 本地站点标识（F20）。
 *
 * 「零网络请求」是硬约束，所以不能去取真实 favicon —— 那既会把用户访问过的
 * 站点泄露出去，也会让预览区依赖网络。退而求其次：从域名算出一个**稳定**的
 * 首字母 + 配色，让不同站点在预览区一眼可分。
 *
 * 稳定很重要：同一个域名每次都要长一样，否则用户会把「颜色变了」当成
 * 「这条记录变了」。所以哈希用确定性的 FNV-1a，配色取自设计系统里已有的
 * 分组色板（不新增颜色，见《前端视觉规范.md》）。
 */

/** 8 个分组色，与 tokens.css 的 --pl-group-* 一一对应。 */
const PALETTE = ["mint", "sky", "indigo", "lotus", "clay", "amber", "moss", "graphite"] as const;

export interface SiteBadge {
  /** 显示用的单字符（域名首字母，大写）。 */
  letter: string;
  /** 直接可用的 CSS 颜色值，如 `var(--pl-group-sky)`。 */
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

/** 从 URL 里取出主机名。不是合法 URL 就返回原串（调用方照旧展示）。 */
export function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url.trim();
  }
}

export function siteBadge(url: string): SiteBadge {
  const host = hostOf(url).toLowerCase();
  // `www.` 不参与取字母和配色 —— 否则 www.github.com 和 github.com 会长得不一样
  const bare = host.replace(/^www\./, "");
  const letter =
    [...bare].find((c) => /[a-z0-9\u4e00-\u9fa5]/i.test(c))?.toUpperCase() ?? "?";
  const color = `var(--pl-group-${PALETTE[fnv1a(bare) % PALETTE.length]})`;
  return { letter, color };
}
