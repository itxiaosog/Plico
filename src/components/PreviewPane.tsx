import { useEffect, useMemo, useState } from "react";

import { detectCode } from "../api";
import { baseName, cssColor, formatFullTime, markdownLink, parseFileList } from "../lib/format";
import { imageUrl } from "../lib/bridge";
import { tokenize } from "../lib/code";
import { t } from "../lib/i18n";
import { buildRichPreviewDoc } from "../lib/richtext";
import { highlight } from "../lib/highlight";
import { hostOf, siteBadge } from "../lib/site";
import { useClipboardStore } from "../store/useClipboardStore";
import { typeLabel, type Item } from "../types";
import { colorFormats } from "../lib/color";
import { useCopyFeedback } from "../lib/useCopyFeedback";
import { TypeIcon } from "./TypeIcon";

function Hl({ text, query }: { text: string; query: string }) {
  return (
    <>
      {highlight(text, query).map((seg, i) =>
        seg.hit ? <mark key={i}>{seg.text}</mark> : <span key={i}>{seg.text}</span>,
      )}
    </>
  );
}

function RichHtml({ html }: { html: string }) {
  // srcDoc 是独立文档，颜色由 buildRichPreviewDoc 显式注入（单一像素主题，固定色值）。
  const srcDoc = useMemo(() => buildRichPreviewDoc(html), [html]);
  return (
    <iframe
      className="pl-preview__html"
      // 空字符串 = 全部限制：禁脚本/表单/弹窗/导航，独立不透明源
      sandbox=""
      srcDoc={srcDoc}
      title={t("previewRichText")}
    />
  );
}

function Body({ item, query }: { item: Item; query: string }) {
  const text = item.plain_text ?? item.content ?? "";
  const copy = useCopyFeedback();
  const [codeLang, setCodeLang] = useState<string | null>(null);
  // 带 HTML 的富文本直接渲染格式，不再对纯文本判代码
  const showHtml = item.type === "rich_text" && !!item.html_content;

  // F20：只对「普通文本/富文本」判代码，其他类型各走各的分支。
  // 检测走 IPC（mock 环境也有），别在前端复制一遍信号表。
  useEffect(() => {
    let dead = false;
    if (!showHtml && (item.type === "text" || item.type === "rich_text")) {
      void detectCode(text)
        .then((lang) => {
          if (!dead) setCodeLang(lang);
        })
        .catch(() => {
          if (!dead) setCodeLang(null);
        });
    } else {
      setCodeLang(null);
    }
    return () => {
      dead = true;
    };
  }, [item.id, item.type, text, showHtml]);

  switch (item.type) {
    case "color": {
      const color = cssColor(text);
      const rows = colorFormats(text);
      return (
        <>
          {color && <div className="pl-swatch pl-swatch--large" style={{ background: color }} />}
          <div style={{ marginTop: 14 }}>
            {rows.length > 0 ? (
              rows.map(([k, v]) => (
                <div className="pl-preview__row" key={k}>
                  <span>{k}</span>
                  <span className="pl-preview__mono">{v}</span>
                  <button type="button" className="pl-link-preview__copy pl-color-copy" disabled={copy.pending} aria-label={`${t("copyValue")} ${k}`} onClick={() => void copy.run(() => v)}>{t("copyValue")}</button>
                </div>
              ))
            ) : (
              <p className="pl-preview__text pl-preview__mono">{text}</p>
            )}
          </div>
          <p role="status" className="pl-copy-feedback" data-error={copy.status === "error"}>{copy.message}</p>
        </>
      );
    }

    case "link": {
      const domain = hostOf(text);
      const badge = siteBadge(text);
      return (
        <>
          <div className="pl-link-preview__header">
            {/* F20：真实 favicon 要联网取，与「零网络请求」冲突，所以用
                域名首字母 + 稳定配色代替 —— 同样能一眼区分不同站点。 */}
            <span
              className="pl-link-preview__favicon"
              aria-hidden="true"
              style={{ color: badge.color, background: "var(--pl-bg-subtle)" }}
            >
              {badge.letter}
            </span>
            <p className="pl-preview__text" style={{ fontSize: 15, fontWeight: 500 }}>
              <Hl text={domain} query={query} />
            </p>
          </div>
          <p className="pl-preview__text pl-preview__mono" style={{ marginTop: 8 }}>
            <Hl text={text} query={query} />
          </p>
          <button
            type="button"
            className="pl-link-preview__copy"
            disabled={copy.pending}
            onClick={() => void copy.run(() => markdownLink(text, item.source_title))}
          >
            {t("copyMarkdown")}
          </button>
          <p role="status" className="pl-copy-feedback" data-error={copy.status === "error"}>{copy.message}</p>
        </>
      );
    }

    case "files": {
      const files = parseFileList(item.content ?? item.plain_text);
      if (files.length === 0) {
        return <p className="pl-preview__hint">{t("previewEmptyFiles")}</p>;
      }
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {files.map((p, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ color: "var(--pl-text-tertiary)", display: "flex" }}>
                <TypeIcon type="files" size={12} />
              </span>
              <span className="pl-preview__text" style={{ minWidth: 0, wordBreak: "break-all" }}>
                {baseName(p)}
              </span>
            </div>
          ))}
        </div>
      );
    }

    case "image": {
      const src = imageUrl(item.image_path);
      return (
        <>
          {src ? (
            <img className="pl-preview__img" src={src} alt={baseName(item.image_path ?? "image")} />
          ) : (
            <p className="pl-preview__hint">{t("previewImageMissing")}</p>
          )}
          <p className="pl-preview__text pl-preview__mono" style={{ marginTop: 8 }}>
            {baseName(item.image_path ?? text)}
          </p>
        </>
      );
    }

    default: {
      if (showHtml) {
        return <RichHtml html={item.html_content!} />;
      }
      // 像代码就切等宽 + 简易着色（F20）；不像就按普通文本渲染。
      if (codeLang) {
        return (
          <pre className="pl-code">
            {tokenize(text, codeLang).map((tok, i) => (
              <span key={i} className={`pl-code__${tok.kind}`}>
                {tok.text}
              </span>
            ))}
          </pre>
        );
      }
      return (
        <p className="pl-preview__text">
          <Hl text={text || t("emptyTitle")} query={query} />
        </p>
      );
    }
  }
}

export function PreviewPane() {
  const items = useClipboardStore((s) => s.items);
  const selectedItemId = useClipboardStore((s) => s.selectedItemId);
  const query = useClipboardStore((s) => s.query);

  const item = items.find((i) => i.id === selectedItemId) ?? null;

  if (!item) {
    return (
      <div className="pl-preview">
        <span className="pl-preview__hint">{t("previewHint")}</span>
      </div>
    );
  }

  return (
    <div className="pl-preview">
      <div className="pl-preview__meta">
        {typeLabel(item.type)} · {item.source_app ?? t("unknownSource")}
      </div>
      <div className="pl-preview__time">{formatFullTime(item.last_copied_at)}</div>
      <Body key={`${item.id}:${item.content}:${item.plain_text}`} item={item} query={query} />
    </div>
  );
}
