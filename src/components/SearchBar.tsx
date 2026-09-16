import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauri, listen } from "../lib/bridge";
import { t } from "../lib/i18n";
import { DEFAULT_GROUP_COLOR, groupDotColor } from "../lib/palette";
import { useClipboardStore } from "../store/useClipboardStore";
import { FILTER_ORDER, filterLabel, type Group } from "../types";
import { ColorPicker } from "./ColorPicker";
import { PencilIcon, SearchIcon, TrashIcon } from "./TypeIcon";

/**
 * 分组菜单里「正在编辑哪一行」的状态。
 *
 * 同一时刻只允许一行处于编辑态（重命名 / 删除确认 / 新建三选一），
 * 与 F17 标签页同一立场：同时开两个输入框，用户分不清哪个提交的是哪个。
 */
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
  /** 排序模式：分组行左侧的圆点变成 ↑/↓ 按钮。与 `editing` 互斥。 */
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

  // 面板每次唤起都要把焦点放回搜索框（F4）
  useEffect(() => {
    const focus = () => inputRef.current?.focus();
    focus();
    const un = listen("plico://panel-shown", focus);
    return () => {
      void un.then((f) => f());
    };
  }, []);

  // 点击别处收起筛选菜单。编辑态也要一并清掉 —— 否则下次打开菜单会
  // 看见上次没收起的输入框，像是卡住了。
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

  // 分组过滤也得体现在按钮上：它会把列表砍到只剩该分组（+ 置顶），
  // 不显示的话用户完全看不出列表为什么变了 —— 「切筛选没反应」的错觉就是这么来的。
  const selectedGroup =
    selectedGroupId === null ? null : (groups.find((g) => g.id === selectedGroupId) ?? null);
  const filterActive = filter !== "all" || selectedGroupId !== null;
  const filterBtnLabel =
    selectedGroup === null
      ? filterLabel(filter)
      : filter === "all"
        ? selectedGroup.name
        : `${selectedGroup.name}·${filterLabel(filter)}`;

  /** 菜单里的所有「收起」动作都走这里，免得漏掉某一处。 */
  const close = () => {
    setOpen(false);
    setEditing(null);
    setSorting(false);
    inputRef.current?.focus();
  };

  const startRename = (group: Group) => {
    // 改名的输入框会占掉整行，排序按钮就没地方放了。两个模式互斥，
    // 开一个就关另一个 —— 和「同一时刻只开一行编辑器」是同一个道理。
    setSorting(false);
    setEditing({ kind: "rename", id: group.id, draft: group.name, color: group.color });
  };

  /**
   * 把第 `index` 个分组往 `delta`（-1 上移 / +1 下移）挪一格。
   *
   * 提交的是**重排后的完整 id 顺序**，不是增量指令 —— 后端据此把 `sort_order`
   * 重写成 `0..n-1`。失败时（多半是列表在别处被改过）`reorderGroups` 已经
   * 把库里的真值拉回来了，这里只需退出排序模式，免得用户对着一份作废的顺序
   * 继续点。
   */
  const moveGroup = (index: number, delta: number) => {
    const target = index + delta;
    if (target < 0 || target >= groups.length) return;
    // 用 `splice` 搬一格，而不是「交换两个位置」：交换写法
    // （`[a[i], a[t]] = [a[t], a[i]]`）在 `noUncheckedIndexedAccess` 下会把两个
    // 元素都读成 `number | undefined`，得靠 `!` 压下去；splice 只取一个返回值，
    // 顺势判一下 `undefined` 就行 —— 正好和上面的边界检查重合。
    const ids = groups.map((g) => g.id);
    const moved = ids.splice(index, 1)[0];
    if (moved === undefined) return;
    ids.splice(target, 0, moved);
    void reorderGroups(ids).catch(() => setSorting(false));
  };

  /**
   * 提交改名的公共部分。
   *
   * 失败时**不收起输入框**：撞名是最常见的失败，用户想接着改一个字再提交，
   * 收起就得从头点一遍。错误文案由 store 塞进 `error`，面板顶部会显示。
   */
  const submitName = async (run: () => Promise<void>) => {
    try {
      await run();
      setEditing(null);
    } catch {
      // 已由 store 写入 error，这里只需保持输入框展开
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
      // 名字和颜色都没动就直接收起，不必往后端跑一趟
      if (!name) return;
      if (name === group.name && editing.color === group.color) { setEditing(null); return; }
      void submitName(() => renameGroup(editing.id, name, editing.color));
    }
  };

  /** 编辑态下键盘的公共处理：Enter 提交、Esc 收起这行（不是收起整个菜单）。 */
  const onEditKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      onSubmitEditing();
    } else if (e.key === "Escape") {
      e.preventDefault();
      // 只退这一步编辑，菜单留着 —— 用户多半是想换个操作
      setEditing(null);
    }
  };

  return (
    <div
      className="pl-search"
      // 搜索栏作为窗口拖动区（面板无边框，没有标题栏可拖）。
      // mousedown 时判断目标：input / button / 菜单内交互不触发拖动，
      // 空白处和搜索图标可以拖。
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
        // 输入法组合期间不触发过滤，否则打中文会闪烁丢字（R6）
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
                    {/* 色板单独占一行：和输入框、两个按钮挤在一行时，
                        输入框会被压到几乎看不见（实测只剩 14px）。 */}
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
                          // 删完收菜单：这一行的上下文已经没了
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
                      // 排序模式下行点击没有「选中分组」的含义（整行是拖拽把手），
                      // 点空处不动选择，只能靠 ↑/↓ 或「完成」退出来。
                      if (sorting) return;
                      setSelectedGroup(group.id);
                      close();
                    }}
                  >
                    {sorting ? (
                      // 排序模式下圆点让位给上下移按钮。**首行的 ↑ 和末行的 ↓ 直接禁用**
                      // 而不是隐藏：藏了整行宽度会跳，连点几下就会点错按钮。
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

            {/* 排序入口只在有 2 个以上分组时才出现：一个分组没什么可排的，
                摆着只是多一行噪音。 */}
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
