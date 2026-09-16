export const CURSOR_PLACEHOLDER = "{{cursor}}";

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

const CLUSTER_EXTEND = /[\p{M}\u200D\uFE0E\uFE0F\p{Emoji_Modifier}]/u;
const REGIONAL_INDICATOR = /[\u{1F1E6}-\u{1F1FF}]/u;

function graphemeCountFallback(text: string): number {
  let count = 0;
  let afterJoiner = false;
  let pendingRegional = false;
  for (const ch of text) {
    if (pendingRegional && REGIONAL_INDICATOR.test(ch)) {
      pendingRegional = false;
      continue;
    }
    if (afterJoiner) {
      afterJoiner = false;
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

export function graphemeCount(text: string): number {
  if (!segmenter) return graphemeCountFallback(text);
  return Array.from(segmenter.segment(text)).length;
}

export function splitCursorPlaceholder(content: string): { text: string; leftMoves: number } {
  const idx = content.indexOf(CURSOR_PLACEHOLDER);
  if (idx < 0) return { text: content, leftMoves: 0 };
  const after = content.slice(idx + CURSOR_PLACEHOLDER.length);
  return {
    text: content.slice(0, idx) + after,
    leftMoves: graphemeCount(after),
  };
}

export function stripCursorPlaceholder(content: string): string {
  return content.split(CURSOR_PLACEHOLDER).join("");
}
