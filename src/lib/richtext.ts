const FG = "#161b19";

export function buildRichPreviewDoc(html: string): string {
  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'">
<style>
  html, body {
    margin: 0;
    padding: 0;
    background: transparent;
    color: ${FG};
    font: 13px/1.7 "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
    word-break: break-word;
    overflow-wrap: anywhere;
  }
  img, video, table { max-width: 100%; }
  pre, code { font-family: Consolas, "Courier New", monospace; }
  table { border-collapse: collapse; }
  td, th { border: 1px solid currentColor; padding: 2px 6px; }
  a { color: inherit; pointer-events: none; }
  blockquote { margin: 8px 0; padding-left: 10px; border-left: 3px solid currentColor; opacity: .9; }
</style>
</head>
<body>${html}</body>
</html>`;
}
