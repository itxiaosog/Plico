import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSnippet, deleteSnippet, listSnippets, pasteSnippet, updateSnippet } from "../../api";
import { isTauri } from "../../lib/bridge";
import type { Snippet } from "../../types";
import { PixelButton, PixelInput, PixelTextarea } from "@pxlkit/ui-kit";
import { Button, ErrorBanner, Section } from "../controls";
import { t } from "../../lib/i18n";

const blank = { title: "", content: "", tags: "", shortcut: "" };

export function SnippetsPage() {
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [draft, setDraft] = useState(blank);
  const [editing, setEditing] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const [deleting, setDeleting] = useState<number | null>(null);

  useEffect(() => {
    let active = true;
    void listSnippets().then((rows) => { if (active) setSnippets(rows); })
      .catch((e: unknown) => { if (active) setError(String(e)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);

  const run = async (action: () => Promise<void>) => {
    if (busy) return;
    setBusy(true); setError(null); setStatus("");
    try { await action(); }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  };
  const reset = () => { setDraft(blank); setEditing(null); };
  const save = () => run(async () => {
    if (!draft.title.trim() || !draft.content.trim()) return;
    const fields = [draft.title.trim(), draft.content, draft.tags.trim() || null, draft.shortcut.trim() || null] as const;
    if (editing === null) await createSnippet(...fields);
    else await updateSnippet(editing, ...fields);
    reset();
    setStatus(t("snippetsSaved"));
    setSnippets(await listSnippets());
  });
  const remove = (id: number) => run(async () => {
    await deleteSnippet(id);
    setSnippets((rows) => rows.filter((s) => s.id !== id));
    if (editing === id) reset();
    setDeleting(null); setStatus(t("snippetsDeleted"));
  });
  const paste = (s: Snippet) => run(async () => {
    if (isTauri) await getCurrentWindow().hide();
    try { await pasteSnippet(s.content); }
    catch (e) {
      if (isTauri) {
        await getCurrentWindow().show();
        await getCurrentWindow().setFocus();
      }
      throw e;
    }
    setStatus(t("snippetsPasted"));
  });

  return <>
    {error && <ErrorBanner text={error} onClose={() => setError(null)} />}
    <div role="status" className="pl-snippets-status">{status}</div>
    <Section title={t("snippetsTitle")}>
      <div className="pl-set-note"><p>{t("snippetsPasteHint")}</p><p>{t("snippetsShortcutHint")}</p></div>
      <div className="pl-set-snippets" aria-busy={busy || loading}>
        {loading ? <div className="pl-set-note">{t("snippetsLoading")}</div> : snippets.length === 0 && !error && <div className="pl-set-note">{t("snippetsEmpty")}</div>}
        {snippets.map(s => <article className="pl-set-snippet" key={s.id}>
          <div className="pl-set-snippet__body"><strong>{s.title}</strong><pre>{s.content}</pre>{(s.tags || s.shortcut) && <small>{[s.tags, s.shortcut].filter(Boolean).join(" · ")}</small>}</div>
          <div className="pl-set-snippet__actions">
            <Button disabled={busy} onClick={() => void paste(s)}>{t("snippetsPaste")}</Button>
            <Button disabled={busy} onClick={() => { setEditing(s.id); setDraft({ title: s.title, content: s.content, tags: s.tags ?? "", shortcut: s.shortcut ?? "" }); setDeleting(null); setStatus(""); }}>{t("snippetsEdit")}</Button>
            <Button disabled={busy} variant="danger" onClick={() => setDeleting(s.id)}>{t("snippetsDelete")}</Button>
          </div>
          {deleting === s.id && <div className="pl-snippets-confirm"><span>{t("snippetsDeleteConfirm")}</span><Button disabled={busy} variant="danger" onClick={() => void remove(s.id)}>{t("clearConfirm")}</Button><Button disabled={busy} onClick={() => setDeleting(null)}>{t("snippetsCancel")}</Button></div>}
        </article>)}
      </div>
    </Section>
    <Section title={editing === null ? t("snippetsCreate") : t("snippetsEdit")}>
      <form className="pl-set-snippet-form" onSubmit={(e) => { e.preventDefault(); void save(); }}>
        <fieldset disabled={busy || loading}>
          <PixelInput label={t("snippetsTitlePlaceholder")} required value={draft.title} onChange={e => setDraft({ ...draft, title: e.target.value })} />
          <PixelTextarea label={t("snippetsContentPlaceholder")} required rows={6} value={draft.content} onChange={e => setDraft({ ...draft, content: e.target.value })} />
          <PixelInput label={t("snippetsTagsPlaceholder")} value={draft.tags} onChange={e => setDraft({ ...draft, tags: e.target.value })} />
          <PixelInput label={t("snippetsShortcutPlaceholder")} value={draft.shortcut} onChange={e => setDraft({ ...draft, shortcut: e.target.value })} />
          <div className="pl-set-snippet__actions"><PixelButton type="submit" variant="solid" tone="green" disabled={!draft.title.trim() || !draft.content.trim()}>{t("snippetsSave")}</PixelButton><Button onClick={reset}>{t("snippetsCancel")}</Button></div>
        </fieldset>
      </form>
    </Section>
  </>;
}
