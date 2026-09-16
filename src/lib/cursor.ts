/**
 * F18 `{{cursor}}` 占位符解析 —— Rust 侧 `clipboard/paste.rs::split_cursor_placeholder`
 * 的镜像实现。
 *
 * 为什么要有一份 JS 的：浏览器预览里没有 Tauri，粘贴走 `mock.ts`。两边语义
 * 不一致的话，预览里看到的行为就不是真机的行为（和 `code.ts` 对 `detect_code`
 * 的作用一样）。
 *
 * **左移次数按字素簇数，不是 `[...text].length`**：粘贴后是靠模拟 N 次 Left
 * 方向键把光标挪回占位符处的，而方向键在编辑器里按「一个视觉字符」走。
 * 下面这些场景码点数与视觉字符数会差出来，按码点数会多按几次、光标落到
 * 占位符**左边**：
 *
 * | 文本 | 码点数 | 视觉字符数 |
 * | --- | --- | --- |
 * | `👨‍👩‍👧`（ZWJ 序列） | 5 | 1 |
 * | `e` + U+0301（组合字符） | 2 | 1 |
 * | `🇨🇳`（两个 regional indicator） | 2 | 1 |
 * | `👍🏽`（+ 肤色修饰符） | 2 | 1 |
 *
 * `Intl.Segmenter` 是现代浏览器 / Node 内置的字素簇切分器，与 Rust 的
 * `unicode-segmentation` 同跟 UAX #29。理论上两者的 Unicode 版本可能差一点，
 * 但那只影响预览 —— 真机永远走 Rust。
 */

export const CURSOR_PLACEHOLDER = "{{cursor}}";

/**
 * `Intl.Segmenter` 是 ES2022 的 API，而 `tsconfig.json` 的 `lib` 停在 ES2020。
 * 这里补一个最小的结构类型，而不是把 `ES2022.Intl` 加进 `lib` —— 后者会顺带
 * 让整个项目都能引用 ES2022 的类型，而 `target` 还是 ES2020，容易掩盖运行时
 * 差异。运行时支持没问题：面板跑在 WebView2（Chromium），Segmenter 从 87 起就有。
 */
interface GraphemeSegmenter {
  segment(input: string): Iterable<{ segment: string }>;
}
interface SegmenterCtor {
  new (locales?: string | string[], options?: { granularity: "grapheme" }): GraphemeSegmenter;
}
const SegmenterImpl = (Intl as unknown as { Segmenter?: SegmenterCtor }).Segmenter;
const segmenter: GraphemeSegmenter | null = SegmenterImpl
  ? new SegmenterImpl(undefined, { granularity: "grapheme" })
  : null;

/** 会把「自己」并进前一个字素簇的码点：组合符号、ZWJ、变体选择符、肤色修饰符。 */
const CLUSTER_EXTEND = /[\p{M}\u200D\uFE0E\uFE0F\p{Emoji_Modifier}]/u;
/** regional indicator：两个连在一起才是一面旗帜。 */
const REGIONAL_INDICATOR = /[\u{1F1E6}-\u{1F1FF}]/u;

/**
 * 无 `Intl.Segmenter` 时的兜底（Chromium ≥ 87 / Node ≥ 16 都有，实际走不到）。
 * 覆盖上表四种情况，够用；不追求 UAX #29 的完整实现。
 */
function graphemeCountFallback(text: string): number {
  let count = 0;
  let afterJoiner = false;
  let pendingRegional = false;
  for (const ch of text) {
    if (pendingRegional && REGIONAL_INDICATOR.test(ch)) {
      pendingRegional = false; // 旗帜的后半段
      continue;
    }
    if (afterJoiner) {
      afterJoiner = false; // ZWJ 后面紧跟的那个码点
      continue;
    }
    if (CLUSTER_EXTEND.test(ch)) {
      afterJoiner = ch === "\u200D";
      continue;
    }
    count += 1;
    pendingRegional = REGIONAL_INDICATOR.test(ch);
  }
  return count;
}

/** 一段文本有几个「视觉字符」。 */
export function graphemeCount(text: string): number {
  if (!segmenter) return graphemeCountFallback(text);
  return Array.from(segmenter.segment(text)).length;
}

/**
 * 把含占位符的片段内容拆成「要粘的纯文本」+「粘贴后需要左移的视觉字符数」。
 *
 * 只处理**第一个**占位符，多余的按普通文本留在内容里 —— 光标只能停一处，
 * 留原文比悄悄删字更不容易误解。
 */
export function splitCursorPlaceholder(content: string): { text: string; leftMoves: number } {
  const idx = content.indexOf(CURSOR_PLACEHOLDER);
  if (idx < 0) return { text: content, leftMoves: 0 };
  const after = content.slice(idx + CURSOR_PLACEHOLDER.length);
  return {
    text: content.slice(0, idx) + after,
    leftMoves: graphemeCount(after),
  };
}

/** 列表里展示片段时把占位符去掉（全部去掉，不是只去第一个）。 */
export function stripCursorPlaceholder(content: string): string {
  return content.split(CURSOR_PLACEHOLDER).join("");
}
