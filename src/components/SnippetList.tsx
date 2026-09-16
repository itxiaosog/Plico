import { useEffect } from "react";
import { t } from "../lib/i18n";
import { stripCursorPlaceholder } from "../lib/cursor";
import { useClipboardStore } from "../store/useClipboardStore";

export function SnippetList() {
  const query = useClipboardStore((s) => s.query);
  const snippets = useClipboardStore((s) => s.snippets);
  const refresh = useClipboardStore((s) => s.refreshSnippets);
  const selectedSnippetId = useClipboardStore((s) => s.selectedSnippetId);
  useEffect(() => { void refresh(); }, [refresh]);
  const term = query.slice(1).trim().toLocaleLowerCase();
  const rows = snippets.filter((s) => !term || `${s.title}\n${s.content}`.toLocaleLowerCase().includes(term));
  useEffect(() => {
    if (rows.length && !rows.some((s) => s.id === selectedSnippetId)) useClipboardStore.setState({ selectedSnippetId: rows[0]?.id ?? null });
  }, [rows, selectedSnippetId]);
  return <div className="pl-list pl-snippet-list">
    <div className="pl-section">{t("snippetsTitle")}</div>
    {rows.length === 0 ? <div className="pl-empty"><span className="pl-empty__title">{t("snippetsEmpty")}</span></div> : rows.map((s) => <button type="button" key={s.id} data-id={s.id} className={`pl-snippet-row${s.id === selectedSnippetId ? " is-selected" : ""}`} onClick={() => useClipboardStore.setState({ selectedSnippetId: s.id })} onDoubleClick={() => void useClipboardStore.getState().pasteSnippet(s.content)}>
      <strong>{s.title}</strong><span>{stripCursorPlaceholder(s.content)}</span>
    </button>)}
  </div>;
}
