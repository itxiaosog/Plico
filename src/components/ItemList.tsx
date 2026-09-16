import { useEffect, useRef, useState, type DragEvent } from "react";

import { useClipboardStore } from "../store/useClipboardStore";
import { useCopyFeedback } from "../lib/useCopyFeedback";
import { t } from "../lib/i18n";
import { markdownLink } from "../lib/format";
import { groupDotColor } from "../lib/palette";
import { GroupDropBar } from "./GroupDropBar";
import { ItemRow } from "./ItemRow";

const DRAG_MIME = "application/x-plico-items";

export function ItemList() {
  const copy = useCopyFeedback();
  const items = useClipboardStore((s) => s.items);
  const selectedItemId = useClipboardStore((s) => s.selectedItemId);
  const selectedIds = useClipboardStore((s) => s.selectedIds);
  const query = useClipboardStore((s) => s.query);
  const filter = useClipboardStore((s) => s.filter);
  const selectedGroupId = useClipboardStore((s) => s.selectedGroupId);
  const pinItem = useClipboardStore((s) => s.pinItem);
  const deleteItem = useClipboardStore((s) => s.deleteItem);
  const deleteSelection = useClipboardStore((s) => s.deleteSelection);
  const clearMultiSelect = useClipboardStore((s) => s.clearMultiSelect);
  const assignGroupToSelection = useClipboardStore((s) => s.assignGroupToSelection);
  const tags = useClipboardStore((s) => s.tags);
  const refreshTags = useClipboardStore((s) => s.refreshTags);
  useEffect(() => { void refreshTags(); }, [refreshTags]);
  const groups = useClipboardStore((s) => s.groups);
  const addItemTag = useClipboardStore((s) => s.addItemTag);
  const [tagItem, setTagItem] = useState<number | null>(null);
  const [groupItem, setGroupItem] = useState<number | null>(null);
  const [newTag, setNewTag] = useState("");
  const [dragCount, setDragCount] = useState(0);

  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (selectedItemId === null) return;
    const el = listRef.current?.querySelector<HTMLElement>(`[data-id="${selectedItemId}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedItemId]);

  const multi = selectedIds.length > 1;

  const pinned = items.filter((i) => i.pinned);
  const recent = items.filter((i) => !i.pinned);

  const onDragStart = (e: DragEvent<HTMLDivElement>) => {
    const row = (e.target as HTMLElement).closest<HTMLElement>("[data-id]");
    if (!row?.dataset.id) return;
    const id = Number(row.dataset.id);
    const st = useClipboardStore.getState();
    const ids = st.selectedIds.includes(id) ? st.selectedIds : [id];
    e.dataTransfer.setData(DRAG_MIME, JSON.stringify(ids));
    e.dataTransfer.effectAllowed = "move";
    setDragCount(ids.length);
  };

  const onDragEnd = () => setDragCount(0);

  const handleDropAssign = (groupId: number | null) => {
    setDragCount(0);
    void assignGroupToSelection(groupId);
  };

  const assignGroupFor = (itemId: number, groupId: number | null) => {
    const st = useClipboardStore.getState();
    if (st.selectedIds.length > 1 && st.selectedIds.includes(itemId)) {
      void st.assignGroupToSelection(groupId);
    } else {
      void st.assignItemGroup(itemId, groupId);
    }
  };

  const renderRow = (id: number) => {
    const item = items.find((i) => i.id === id);
    if (!item) return null;
    return (
      <ItemRow
        key={item.id}
        item={item}
        selected={item.id === selectedItemId}
        multiSelected={multi && selectedIds.includes(item.id)}
        query={query}
        draggable
        onSelect={() => useClipboardStore.getState().selectOnly(item.id)}
        onToggleMulti={() => useClipboardStore.getState().toggleSelected(item.id)}
        onRangeSelect={() => useClipboardStore.getState().selectRange(item.id)}
        onPin={() => void pinItem(item.id)}
        onDelete={() => void deleteItem(item.id)}
        onPastePlain={() => void useClipboardStore.getState().pasteItem(item.id, true)}
        onCopyMarkdown={
          item.type === "link"
            ? () => void copy.run(() => markdownLink(item.plain_text ?? item.content ?? "", item.source_title))
            : undefined
        }
        onAddTag={() => setTagItem(item.id)}
        onAssignGroup={() => setGroupItem(item.id)}
      />
    );
  };

  if (items.length === 0) {
    const searching = query.trim().length > 0 || filter !== "all";
    const inGroup = selectedGroupId !== null;
    return (
      <div className="pl-list">
        <div className="pl-empty">
          <span className="pl-empty__title">
            {searching ? t("emptySearchTitle") : inGroup ? t("groupEmptyTitle") : t("emptyTitle")}
          </span>
          <span className="pl-empty__hint">
            {searching ? t("emptySearchHint") : inGroup ? t("groupEmptyHint") : t("emptyHint")}
          </span>
        </div>
      </div>
    );
  }

  return (
    <>
      {tagItem !== null && <div className="pl-dialog" role="dialog"><strong>{t("addTag")}</strong>{tags.map(tag => <button type="button" className="pl-menu__item" key={tag.id} onClick={() => { void addItemTag(tagItem, tag.name); setTagItem(null); }}>{tag.name}</button>)}<input autoFocus value={newTag} placeholder={t("tagPrompt")} onChange={e => setNewTag(e.target.value)} onKeyDown={e => { if (e.key === "Enter" && newTag.trim()) { void addItemTag(tagItem, newTag.trim()); setNewTag(""); setTagItem(null); } }} /><button type="button" onClick={() => setTagItem(null)}>{t("snippetsCancel")}</button></div>}
      {groupItem !== null && <div className="pl-dialog" role="dialog"><strong>{t("groupAssign")}</strong><button type="button" className="pl-menu__item" onClick={() => { assignGroupFor(groupItem, null); setGroupItem(null); }}>{t("groupNone")}</button>{groups.map(group => <button type="button" className="pl-menu__item" key={group.id} onClick={() => { assignGroupFor(groupItem, group.id); setGroupItem(null); }}><span className="pl-group-dot" style={{ background: groupDotColor(group.color) }} />{group.name}</button>)}<button type="button" onClick={() => setGroupItem(null)}>{t("snippetsCancel")}</button></div>}
      <div className="pl-list" ref={listRef} onDragStart={onDragStart} onDragEnd={onDragEnd}>
      <p role="status" className="pl-copy-feedback pl-copy-feedback--list" data-error={copy.status === "error"}>{copy.message}</p>

      {dragCount > 0 ? (
        <div className="pl-list-head">
          <GroupDropBar groups={groups} count={dragCount} onAssign={handleDropAssign} />
        </div>
      ) : multi ? (
        <div className="pl-list-head">
          <div className="pl-multi-bar">
            <span className="pl-multi-bar__count">{t("multiSelected")} {selectedIds.length} {t("unitItems")}</span>
            <button type="button" className="pl-mini-btn" onClick={() => setGroupItem(selectedItemId)}>{t("multiAssign")}</button>
            <button type="button" className="pl-mini-btn pl-mini-btn--danger" onClick={() => void deleteSelection()}>{t("multiDelete")}</button>
            <button type="button" className="pl-mini-btn" onClick={() => clearMultiSelect()}>{t("multiClear")}</button>
          </div>
          <div className="pl-multi-hint">{t("multiHint")}</div>
        </div>
      ) : null}

      {pinned.length > 0 && (
        <>
          <div className="pl-section">{t("sectionPinned")}</div>
          {pinned.map((i) => renderRow(i.id))}
        </>
      )}

      {recent.length > 0 && (
        <>
          <div className={`pl-section${pinned.length > 0 ? " pl-section--divided" : ""}`}>
            {pinned.length > 0 ? t("sectionRecent") : t("sectionAll")}
          </div>
          {recent.map((i) => renderRow(i.id))}
        </>
      )}
      </div>
    </>
  );
}
