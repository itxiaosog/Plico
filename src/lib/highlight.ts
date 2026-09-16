export interface Segment {
  text: string;
  hit: boolean;
}

export function highlight(text: string, query: string): Segment[] {
  const q = query.trim();
  if (!q) {
    return [{ text, hit: false }];
  }

  const haystack = text.toLowerCase();
  const needle = q.toLowerCase();

  const segments: Segment[] = [];
  let cursor = 0;

  for (;;) {
    const idx = haystack.indexOf(needle, cursor);
    if (idx < 0) {
      break;
    }
    if (idx > cursor) {
      segments.push({ text: text.slice(cursor, idx), hit: false });
    }
    segments.push({ text: text.slice(idx, idx + needle.length), hit: true });
    cursor = idx + needle.length;
  }

  if (cursor < text.length) {
    segments.push({ text: text.slice(cursor), hit: false });
  }

  return segments.length > 0 ? segments : [{ text, hit: false }];
}
