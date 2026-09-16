import { useEffect, useRef } from "react";

import { t } from "../lib/i18n";
import { useClipboardStore } from "../store/useClipboardStore";

/**
 * F19 条目编辑器。
 *
 * 在面板右侧预览区原位替换：Ctrl+E 进入，Enter 粘贴（Ctrl+Enter 纯文本），
 * Esc 放弃。编辑内容默认不写回库 —— 规格书允许默认不动原记录。
 */
export function EditPane() {
  const editingId = useClipboardStore((s) => s.editingId);
  const draft = useClipboardStore((s) => s.editingDraft);
  const setDraft = useClipboardStore((s) => s.setEditingDraft);
  const items = useClipboardStore((s) => s.items);

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const item = items.find((i) => i.id === editingId) ?? null;

  // 进入编辑模式就把焦点放进 textarea，并选中全部内容
  useEffect(() => {
    if (!item) return;
    const el = textareaRef.current;
    if (!el) return;
    el.focus();
    el.select();
  }, [editingId]); // eslint-disable-line react-hooks/exhaustive-deps -- 只在切入时做一次

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
