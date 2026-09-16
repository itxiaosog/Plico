import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauri, listen } from "../lib/bridge";
import { t } from "../lib/i18n";
import { DEFAULT_GROUP_COLOR, groupDotColor } from "../lib/palette";
import { useClipboardStore } from "../store/useClipboardStore";
import { FILTER_ORDER, filterLabel, type Group } from "../types";
import { ColorPicker } from "./ColorPicker";
import { PencilIcon, SearchIcon, TrashIcon } from "./TypeIcon";

type Editing =
  | { kind: "rename"; id: number; draft: string; color: string | null }
  | { kind: "delete"; id: number }
  | { kind: "create"; draft: string; color: string | null }
  | null;

export function SearchBar() {
  const inputRef = useRef<HTMLInputElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<Editing>(null);
  const [sorting, setSorting] = useState(false);

  const query = useClipboardStore((s) => s.query);
  const setQuery = useClipboardStore((s) => s.setQuery);
  const setComposing = useClipboardStore((s) => s.setComposing);
  const filter = useClipboardStore((s) => s.filter);
  const setFilter = useClipboardStore((s) => s.setFilter);
  const groups = useClipboardStore((s) => s.groups);
  const selectedGroupId = useClipboardStore((s) => s.selectedGroupId);
  const setSelectedGroup = useClipboardStore((s) => s.setSelectedGroup);
  const createGroup = useClipboardStore((s) => s.createGroup);
  const renameGroup = useClipboardStore((s) => s.renameGroup);
  const deleteGroup = useClipboardStore((s) => s.deleteGroup);
  const reorderGroups = useClipboardStore((s) => s.reorderGroups);

  useEffect(() => {
    void useClipboardStore.getState().refreshGroups();
  }, []);

  useEffect(() => {
    const focus = () => inputRef.current?.focus();
    focus();
    const un = listen("plico://panel-shown", focus);
    return () => {
      void un.then((f) => f());
    };
  }, []);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
          if (!menuRef.current?.contains(e.target as Node)) {
        setOpen(false);
        setEditing(null);
        setSorting(false);
      }
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);

  const selectedGroup =
    selectedGroupId === null ? null : (groups.find((g) => g.id === selectedGroupId) ?? null);
  const filterActive = filter !== "all" || selectedGroupId !== null;
  const filterBtnLabel =
    selectedGroup === null
      ? filterLabel(filter)
      : filter === "all"
        ? selectedGroup.name
        : `${selectedGroup.name}·${filterLabel(filter)}`;

  const close = () => {
    setOpen(false);
    setEditing(null);
    setSorting(false);
    inputRef.current?.focus();
  };

  const startRename = (group: Group) => {
    setSorting(false);
    setEditing({ kind: "rename", id: group.id, draft: group.name, color: group.color });
  };

  const moveGroup = (index: number, delta: number) => {
    const target = index + delta;
    if (target < 0 || target >= groups.length) return;
    const ids = groups.map((g) => g.id);
    const moved = ids.splice(index, 1)[0];
    if (moved === undefined) return;
    ids.splice(target, 0, moved);
    void reorderGroups(ids).catch(() => setSorting(false));
  };

  const submitName = async (run: () => Promise<void>) => {
    try {
      await run();
      setEditing(null);
    } catch {
    }
  };

  const onSubmitEditing = () => {
    if (!editing) return;
    if (editing.kind === "create") {
      const name = editing.draft.trim();
      if (!name) return;
      void submitName(() => createGroup(name, editing.color));
    } else if (editing.kind === "rename") {
      const group = groups.find((g) => g.id === editing.id);
      if (!group) { setEditing(null); return; }
      const name = editing.draft.trim();
      if (!name) return;
      if (name === group.name && editing.color === group.color) { setEditing(null); return; }
      void submitName(() => renameGroup(editing.id, name, editing.color));
    }
  };

  const onEditKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      onSubmitEditing();
    } else if (e.key === "Escape") {
      e.preventDefault();
      setEditing(null);
    }
  };

  return (
    <div
      className="pl-search"
      onMouseDown={(e) => {
        if (!isTauri) return;
        const target = e.target as HTMLElement;
        if (target.closest("input, button, .pl-menu, .pl-gedit")) return;
        void getCurrentWindow().startDragging();
      }}
    >
      <span className="pl-search__icon">
        <SearchIcon />
      </span>

      <input
        ref={inputRef}
        className="pl-search__input"
        value={query}
        placeholder={t("searchPlaceholder")}
        spellCheck={false}
        autoComplete="off"
        autoCorrect="off"
        onChange={(e) => setQuery(e.target.value)}
        onCompositionStart={() => setComposing(true)}
        onCompositionEnd={(e) => {
          setComposing(false);
          setQuery(e.currentTarget.value);
        }}
      />

      <div ref={menuRef} style={{ position: "relative" }}>
        <button
          type="button"
          className={`pl-filter${filterActive ? " is-active" : ""}`}
          data-type={filter}
          onClick={() => { setOpen((v) => !v); setEditing(null); setSorting(false); }}
          title={`${t("statusFilter")}（Tab）`}
        >
          {selectedGroup && (
            <span
              className="pl-group-dot"
              style={{ background: groupDotColor(selectedGroup.color) }}
            />
          )}
          {filterBtnLabel}
        </button>

        {open && (
          <div className="pl-menu">
            {FILTER_ORDER.map((k) => (
              <button
                key={k}
                type="button"
                className={`pl-menu__item${k === filter ? " is-active" : ""}`}
                onClick={() => {
                  setFilter(k);
                  close();
                }}
              >
                {filterLabel(k)}
              </button>
            ))}
            <div className="pl-menu__separator" />
            <button
              type="button"
              className={`pl-menu__item${selectedGroupId === null ? " is-active" : ""}`}
              onClick={() => {
                setSelectedGroup(null);
                close();
              }}
            >
              {t("groupAll")}
            </button>

            {groups.map((group, index) => (
              <div className="pl-group-row" key={group.id}>
                {editing?.kind === "rename" && editing.id === group.id ? (
                  <div className="pl-gedit">
                    <div className="pl-gedit__line">
                      <input
                        className="pl-ginput"
                        autoFocus
                        value={editing.draft}
                        placeholder={t("groupRenamePrompt")}
                        spellCheck={false}
                        onChange={(e) => setEditing({ ...editing, draft: e.target.value })}
                        onKeyDown={onEditKeyDown}
                      />
                    </div>

                    <div className="pl-gedit__line">
                      <ColorPicker
                        value={editing.color}
                        onPick={(color) => setEditing({ ...editing, color })}
                      />
                      <span className="pl-gedit__spacer" />
                      <button type="button" className="pl-gbtn is-primary" onClick={onSubmitEditing}>
                        {t("tagsConfirm")}
                      </button>
                      <button type="button" className="pl-gbtn" onClick={() => setEditing(null)}>
                        {t("tagsCancel")}
                      </button>
                    </div>
                  </div>
                ) : editing?.kind === "delete" && editing.id === group.id ? (
                  <div className="pl-gedit">
                    <div className="pl-gedit__line">
                      <span className="pl-gconfirm">{t("groupDeleteConfirm")}</span>
                    </div>
                    <div className="pl-gedit__line pl-gedit__line--end">
                      <button
                        type="button"
                        className="pl-gbtn is-danger"
                        onClick={() => {
                          void deleteGroup(group.id)
                            .then(() => setEditing(null))
                            .catch(() => setEditing(null));
                        }}
                      >
                        {t("groupDelete")}
                      </button>
                      <button type="button" className="pl-gbtn" onClick={() => setEditing(null)}>
                        {t("tagsCancel")}
                      </button>
                    </div>
                  </div>
                ) : (
                  <button
                    type="button"
                    className={`pl-menu__item${selectedGroupId === group.id ? " is-active" : ""}`}
                    onClick={() => {
                      if (sorting) return;
                      setSelectedGroup(group.id);
                      close();
                    }}
                  >
                    {sorting ? (
                      <span className="pl-sort-ctl">
                        <button
                          type="button"
                          className="pl-sort-btn"
                          disabled={index === 0}
                          title={t("groupSortMoveUp")}
                          onClick={(e) => { e.stopPropagation(); moveGroup(index, -1); }}
                        >
                          ↑
                        </button>
                        <button
                          type="button"
                          className="pl-sort-btn"
                          disabled={index === groups.length - 1}
                          title={t("groupSortMoveDown")}
                          onClick={(e) => { e.stopPropagation(); moveGroup(index, 1); }}
                        >
                          ↓
                        </button>
                      </span>
                    ) : (
                      <span
                        className="pl-group-dot"
                        style={{ background: groupDotColor(group.color) }}
                      />
                    )}
                    <span className="pl-group-row__name">{group.name}</span>
                    {!sorting && (
                      <span className="pl-group-actions">
                        <button
                          type="button"
                          className="pl-icon-btn"
                          title={t("groupRename")}
                          aria-label={t("groupRename")}
                          onClick={(e) => { e.stopPropagation(); startRename(group); }}
                        >
                          <PencilIcon />
                        </button>
                        <button
                          type="button"
                          className="pl-icon-btn is-danger"
                          title={t("groupDelete")}
                          aria-label={t("groupDelete")}
                          onClick={(e) => { e.stopPropagation(); setEditing({ kind: "delete", id: group.id }); }}
                        >
                          <TrashIcon />
                        </button>
                      </span>
                    )}
                  </button>
                )}
              </div>
            ))}

            {groups.length >= 2 && (
              <button
                type="button"
                className={`pl-menu__item${sorting ? " is-active" : ""}`}
                onClick={() => { setSorting((v) => !v); setEditing(null); }}
              >
                {sorting ? t("groupSortDone") : t("groupSort")}
              </button>
            )}

            {editing?.kind === "create" ? (
              <div className="pl-gedit">
                <div className="pl-gedit__line">
                  <input
                    className="pl-ginput"
                    autoFocus
                    value={editing.draft}
                    placeholder={t("groupCreatePrompt")}
                    spellCheck={false}
                    onChange={(e) => setEditing({ ...editing, draft: e.target.value })}
                    onKeyDown={onEditKeyDown}
                  />
                </div>
                <div className="pl-gedit__line">
                  <ColorPicker
                    value={editing.color}
                    onPick={(color) => setEditing({ ...editing, color })}
                  />
                  <span className="pl-gedit__spacer" />
                  <button type="button" className="pl-gbtn is-primary" onClick={onSubmitEditing}>
                    {t("tagsConfirm")}
                  </button>
                  <button type="button" className="pl-gbtn" onClick={() => setEditing(null)}>
                    {t("tagsCancel")}
                  </button>
                </div>
              </div>
            ) : (
              <button
                type="button"
                className="pl-menu__item pl-menu__item--create"
                onClick={() => { setSorting(false); setEditing({ kind: "create", draft: "", color: DEFAULT_GROUP_COLOR }); }}
              >
                + {t("groupCreate")}
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
