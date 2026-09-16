import { useEffect, useState } from "react";

import { PixelInput, PixelSelect } from "@pxlkit/ui-kit";
import { deleteTag, listTagStats, mergeTags, renameTag } from "../../api";
import { t } from "../../lib/i18n";
import type { TagStat } from "../../types";
import { Button, ErrorBanner, Section } from "../controls";

export function TagsPage() {
  const [stats, setStats] = useState<TagStat[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState("");

  const [renaming, setRenaming] = useState<number | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  const [merging, setMerging] = useState<number | null>(null);
  const [mergeTarget, setMergeTarget] = useState<number | null>(null);
  const [deleting, setDeleting] = useState<number | null>(null);

  useEffect(() => {
    let active = true;
    void listTagStats()
      .then((rows) => { if (active) setStats(rows); })
      .catch((e: unknown) => { if (active) setError(String(e)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);

  const run = (action: () => Promise<TagStat[]>, done: string) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    setStatus("");
    void action()
      .then((rows) => {
        setStats(rows);
        setRenaming(null);
        setMerging(null);
        setDeleting(null);
        setStatus(done);
      })
      .catch((e: unknown) => setError(String(e)))
      .finally(() => setBusy(false));
  };

  const open = (which: "rename" | "merge" | "delete", tag: TagStat) => {
    setRenaming(which === "rename" ? tag.id : null);
    setMerging(which === "merge" ? tag.id : null);
    setDeleting(which === "delete" ? tag.id : null);
    setStatus("");
    if (which === "rename") setRenameDraft(tag.name);
    if (which === "merge") setMergeTarget(stats.find((s) => s.id !== tag.id)?.id ?? null);
  };

  const submitRename = (tag: TagStat) => {
    const name = renameDraft.trim();
    if (!name || name === tag.name) { setRenaming(null); return; }
    run(() => renameTag(tag.id, name), t("tagsRenamed"));
  };

  return <>
    {error && <ErrorBanner text={error} onClose={() => setError(null)} />}
    <div role="status" className="pl-snippets-status">{status}</div>
    <Section title={t("tagsTitle")}>
      <div className="pl-set-note"><p>{t("tagsHint")}</p></div>
      <div className="pl-set-tags" aria-busy={busy || loading}>
        {loading
          ? <div className="pl-set-note">{t("tagsLoading")}</div>
          : stats.length === 0
            ? <div className="pl-set-note">{t("tagsEmpty")}</div>
            : stats.map((tag) => (
              <div className="pl-set-tag" key={tag.id}>
                <div className="pl-set-tag__main">
                  {renaming === tag.id
                    ? <PixelInput
                        className="pl-set-tag__input"
                        autoFocus
                        size="sm"
                        value={renameDraft}
                        placeholder={t("tagsRenamePlaceholder")}
                        onChange={(e) => setRenameDraft(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") { e.preventDefault(); submitRename(tag); }
                          else if (e.key === "Escape") setRenaming(null);
                        }}
                      />
                    : <span className="pl-set-tag__name">#{tag.name}</span>}
                  <span className="pl-set-tag__count">{tag.count} {t("tagsCountUnit")}</span>
                  <div className="pl-set-tag__actions">
                    {renaming === tag.id ? <>
                      <Button variant="primary" disabled={busy} onClick={() => submitRename(tag)}>{t("tagsConfirm")}</Button>
                      <Button disabled={busy} onClick={() => setRenaming(null)}>{t("tagsCancel")}</Button>
                    </> : <>
                      <Button disabled={busy} onClick={() => open("rename", tag)}>{t("tagsRename")}</Button>
                      <Button disabled={busy || stats.length < 2} onClick={() => open("merge", tag)}>{t("tagsMerge")}</Button>
                      <Button variant="danger" disabled={busy} onClick={() => open("delete", tag)}>{t("tagsDelete")}</Button>
                    </>}
                  </div>
                </div>

                {merging === tag.id && (
                  <div className="pl-set-tag__panel">
                    <span>{t("tagsMergeInto")}</span>
                    <PixelSelect
                      value={mergeTarget === null ? "" : String(mergeTarget)}
                      disabled={busy}
                      options={stats
                        .filter((s) => s.id !== tag.id)
                        .map((s) => ({ value: String(s.id), label: `#${s.name}（${s.count}）` }))}
                      onChange={(v) => setMergeTarget(Number(v))}
                    />
                    <Button
                      variant="primary"
                      disabled={busy || mergeTarget === null}
                      onClick={() => { if (mergeTarget !== null) run(() => mergeTags(tag.id, mergeTarget), t("tagsMerged")); }}
                    >
                      {t("tagsConfirm")}
                    </Button>
                    <Button disabled={busy} onClick={() => setMerging(null)}>{t("tagsCancel")}</Button>
                    <span className="pl-set-tag__hint">{t("tagsMergeHint")}</span>
                  </div>
                )}

                {deleting === tag.id && (
                  <div className="pl-set-tag__panel">
                    <span>{t("tagsDeleteConfirm")}</span>
                    <Button variant="danger" disabled={busy} onClick={() => run(() => deleteTag(tag.id), t("tagsDeleted"))}>
                      {t("tagsDelete")}
                    </Button>
                    <Button disabled={busy} onClick={() => setDeleting(null)}>{t("tagsCancel")}</Button>
                  </div>
                )}
              </div>
            ))}
      </div>
    </Section>
  </>;
}
