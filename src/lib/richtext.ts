/**
 * 富文本预览（F19）：把条目里的 html_content 包成一个可安全渲染的 srcDoc。
 *
 * 安全模型（对应《开发环境说明》§4.4 的前置要求）：
 * - iframe 挂 `sandbox=""`（全禁）：无脚本、无表单、无弹窗、无顶层导航，
 *   且拿到独立不透明源 —— 即使 HTML 里藏了 <script> 也不会执行。
 * - 文档内 CSP `default-src 'none'`：任何子资源（图片/字体/网络请求）都
 *   被拦，clipboard 里的 HTML 引用外部图也加载不出来，保证零网络请求。
 * - 唯一放行的是 `style-src 'unsafe-inline'` —— 富文本的格式本来就靠
 *   内联 style 携带，禁了它预览就退化成纯文本，这个功能就没意义了。
 *
 * srcDoc 是独立文档，外面的 CSS 变量透不进去，所以颜色显式注入。
 * 单一像素主题后只需要一套色值（与 tokens.css 的 --pl-text-primary 对齐）。
 */

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
  /* 链接只展示不跳转 —— sandbox 本来就禁导航，pointer-events 是双保险 */
  a { color: inherit; pointer-events: none; }
  blockquote { margin: 8px 0; padding-left: 10px; border-left: 3px solid currentColor; opacity: .9; }
</style>
</head>
<body>${html}</body>
</html>`;
}
