import { useEffect, useRef } from "react";

import { t } from "../lib/i18n";
import { useClipboardStore } from "../store/useClipboardStore";

export function EditPane() {
  const editingId = useClipboardStore((s) => s.editingId);
  const draft = useClipboardStore((s) => s.editingDraft);
  const setDraft = useClipboardStore((s) => s.setEditingDraft);
  const items = useClipboardStore((s) => s.items);

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const item = items.find((i) => i.id === editingId) ?? null;

  useEffect(() => {
    if (!item) return;
    const el = textareaRef.current;
    if (!el) return;
    el.focus();
    el.select();
  }, [editingId]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!item) return null;

  return (
    <div className="pl-preview pl-edit">
      <div className="pl-preview__meta">
        {t("editTitle")} · {item.source_app ?? t("unknownSource")}
      </div>
      <textarea
        ref={textareaRef}
        className="pl-edit__input"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        spellCheck={false}
      />
      <div className="pl-edit__hint">
        {t("editHint")}
      </div>
    </div>
  );
}
