import { create } from "zustand";

import * as api from "../api";
import { FILTER_ORDER, type FilterKind, type Item, type Stats } from "../types";

interface ClipboardState {
  items: Item[];
  snippets: import("../types").Snippet[];
  query: string;
  filter: FilterKind;
  selectedItemId: number | null;
  /**
   * 多选集合（F16）。**始终包含 `selectedItemId`**，长度 > 1 时才进入「批量模式」。
   *
   * 之所以让光标项和集合分开存：预览区需要且只需要一个「当前项」，而批量动作
   * 需要一整组。合成一个字段就得在每个用到光标的地方都做一次 `[0]`，反而更绕。
   */
  selectedIds: number[];
  /** Shift 连选的锚点。保持不变，所以连续 Shift 点击是「重新从锚点扩选」而不是累加。 */
  selectionAnchorId: number | null;
  selectedSnippetId: number | null;
  stats: Stats;
  loading: boolean;
  /** 输入法组合态。组合期间不触发过滤，否则打中文会闪烁丢字（R6）。 */
  composing: boolean;
  error: string | null;
  groups: import("../types").Group[];
  selectedGroupId: number | null;

  refresh: () => Promise<void>;
  refreshSnippets: () => Promise<void>;
  refreshGroups: () => Promise<void>;
  setSelectedGroup: (id: number | null) => void;
  /**
   * 三个写操作都**向上抛错**，同时把后端原话塞进 `error`。
   * 调用方（内联输入框）需要知道自己这次成没成 —— 撞名时要把输入框留在原地
   * 让用户接着改，而不是当成功一样收起来。
   */
  createGroup: (name: string, color?: string | null) => Promise<void>;
  renameGroup: (id: number, name: string, color?: string | null) => Promise<void>;
  deleteGroup: (id: number) => Promise<void>;
  /**
   * 按完整 id 顺序重排分组（F16 排序 UI）。
   *
   * 和另外三个写操作一样**向上抛错**：拿着过期列表提交时后端会拒绝
   * （「分组列表已变化，请重试」），排序模式要据此退出并让外层重取列表，
   * 而不是默默当作排好了。
   */
  reorderGroups: (ids: number[]) => Promise<void>;
  assignItemGroup: (itemId: number, groupId: number | null) => Promise<void>;
  tags: import("../types").Tag[];
  itemTags: Record<number, import("../types").Tag[]>;
  refreshTags: () => Promise<void>;
  loadItemTags: (itemId: number) => Promise<void>;
  addItemTag: (itemId: number, name: string) => Promise<void>;
  removeItemTag: (itemId: number, name: string) => Promise<void>;
  setQuery: (q: string) => void;
  setComposing: (c: boolean) => void;
  setFilter: (f: FilterKind) => void;
  cycleFilter: (dir: number) => void;
  moveSelection: (delta: number) => void;
  /** 只选中这一条（普通点击）。 */
  selectOnly: (id: number) => void;
  /** Ctrl / Cmd 点击：把这一条加进或移出多选。 */
  toggleSelected: (id: number) => void;
  /** Shift 点击：从锚点连选到这一条。 */
  selectRange: (id: number) => void;
  /** Ctrl+A：全选当前列表里可见的条目。 */
  selectAllVisible: () => void;
  /** 把批量选择收缩回单项（Esc 的第一段行为）。 */
  clearMultiSelect: () => void;
  /** F16：把当前选择（单项或批量）整体归到某个分组，`null` = 移出分组。 */
  assignGroupToSelection: (groupId: number | null) => Promise<void>;
  /** F16：删除当前选择（单项或批量）。 */
  deleteSelection: () => Promise<void>;
  pinItem: (id: number) => Promise<void>;
  deleteItem: (id: number) => Promise<void>;
  removeSelected: () => Promise<void>;
  togglePinSelected: () => Promise<void>;
  pasteSelected: (asPlainText?: boolean) => Promise<void>;
  /** 粘贴指定 id 的条目（上下文菜单用，不一定等于当前选中）。 */
  pasteItem: (id: number, asPlainText?: boolean) => Promise<void>;
  /** 直接粘一段文本（F19 编辑后的内容，不是库里的条目）。asPlainText 时剥离富文本。 */
  pasteText: (text: string, asPlainText?: boolean) => Promise<void>;
  /** F18：粘片段内容，`{{cursor}}` 占位符由后端处理，粘贴后光标停在占位符处。 */
  pasteSnippet: (content: string) => Promise<void>;
  /** 当前是否处于「编辑条目」模式（F19）。 */
  editingId: number | null;
  /** 编辑草稿（编辑器里的当前文本）。 */
  editingDraft: string;
  setEditing: (id: number | null) => void;
  setEditingDraft: (text: string) => void;
  /** Ctrl+E：进入编辑模式，草稿初始化为该条的纯文本。 */
  beginEditSelected: () => void;
  /** 清空搜索词。返回 true 表示确实清掉了（Esc 的两段行为要用）。 */
  clearQuery: () => boolean;
  reset: () => void;
}

let refreshTimer: number | undefined;
/** 请求序号：搜索框每敲一个字都会发请求，必须丢弃过期响应，否则列表会回跳。 */
let requestSeq = 0;

export const useClipboardStore = create<ClipboardState>((set, get) => ({
  items: [],
  snippets: [],
  query: "",
  filter: "all",
  selectedItemId: null,
  selectedIds: [],
  selectionAnchorId: null,
  selectedSnippetId: null,
  stats: { total: 0, pinned: 0 },
  loading: false,
  composing: false,
  error: null,
  groups: [],
  selectedGroupId: null,
  tags: [],
  itemTags: {},

  refreshSnippets: async () => { try { set({ snippets: await api.listSnippets() }); } catch (e) { set({ error: String(e) }); } },

  refresh: async () => {
    const seq = ++requestSeq;
    const { query, filter, selectedGroupId } = get();
    set({ loading: true });

    try {
      const [items, stats] = await Promise.all([
        api.listItems(query, filter),
        api.getStats(),
      ]);

      const visibleItems = selectedGroupId === null ? items : items.filter((i) => i.group_id === selectedGroupId || i.pinned);

      // 已经有更新的请求发出去了，这次结果作废
      if (seq !== requestSeq) return;

      const { selectedItemId, selectedIds } = get();
      const visibleIds = new Set(visibleItems.map((i) => i.id));
      const cursorKept = selectedItemId !== null && visibleIds.has(selectedItemId);

      // 光标项还在 → 保留批量集合（丢掉被过滤掉的那些）；
      // 光标项没了 → 整个选择重置成列表第一条，与「刚打开面板」的行为一致。
      let cursor: number | null;
      let nextSelected: number[];
      if (cursorKept) {
        cursor = selectedItemId;
        const kept = selectedIds.filter((id) => visibleIds.has(id));
        nextSelected = kept.includes(cursor) ? kept : [...kept, cursor];
      } else {
        cursor = visibleItems[0]?.id ?? null;
        nextSelected = cursor === null ? [] : [cursor];
      }

      set({
        items: visibleItems,
        stats,
        loading: false,
        error: null,
        selectedItemId: cursor,
        selectedIds: nextSelected,
      });
    } catch (e) {
      if (seq !== requestSeq) return;
      set({ loading: false, error: String(e) });
    }
  },

  refreshGroups: async () => {
    try { set({ groups: await api.listGroups() }); } catch (e) { set({ error: String(e) }); }
  },
  setSelectedGroup: (id) => { set({ selectedGroupId: id }); void get().refresh(); },
  // 三个写操作都把后端返回的完整列表直接铺上，不自己打补丁。
  // 失败时**往 store 的 error 里塞后端原话**（如「已存在同名分组「工作」」），
  // 面板顶部会把它显示出来 —— 分组菜单本身没有地方展示错误条。
  createGroup: async (name, color) => {
    try { set({ groups: await api.createGroup(name, color) }); }
    catch (e) { set({ error: String(e) }); throw e; }
  },
  renameGroup: async (id, name, color) => {
    try { set({ groups: await api.renameGroup(id, name, color) }); }
    catch (e) { set({ error: String(e) }); throw e; }
  },
  deleteGroup: async (id) => {
    try {
      set({ groups: await api.deleteGroup(id) });
      if (get().selectedGroupId === id) set({ selectedGroupId: null });
      await get().refresh();
    } catch (e) { set({ error: String(e) }); throw e; }
  },
  reorderGroups: async (ids) => {
    // 先乐观铺一遍：排序是「连点几下」的密集操作，等 IPC 回来才动会让按钮明显发滞。
    // 后端成功后会返回权威列表覆盖它；失败时下面的 catch 里再 refresh 拉回真值。
    // 只在「传进来的 id 与本地列表完全对得上」时才铺 —— 对不上说明本地列表已经过期，
    // 硬铺会造出一份凭空少几个分组的中间态，比不铺更吓人。
    const { groups } = get();
    const byId = new Map(groups.map((g) => [g.id, g]));
    const optimistic = ids.length === groups.length
      ? ids.map((id, index) => {
          const g = byId.get(id);
          return g ? { ...g, sort_order: index } : null;
        })
      : [];
    if (optimistic.every((g) => g !== null)) {
      set({ groups: optimistic as import("../types").Group[] });
    }
    try { set({ groups: await api.reorderGroups(ids) }); }
    catch (e) { set({ error: String(e) }); await get().refreshGroups(); throw e; }
  },
  assignItemGroup: async (itemId, groupId) => { try { await api.assignItemGroup(itemId, groupId); await get().refresh(); } catch (e) { set({ error: String(e) }); } },
  refreshTags: async () => { try { set({ tags: await api.listTags() }); } catch (e) { set({ error: String(e) }); } },
  loadItemTags: async (itemId) => { try { set({ itemTags: { ...get().itemTags, [itemId]: await api.getItemTags(itemId) } }); } catch (e) { set({ error: String(e) }); } },
  addItemTag: async (itemId, name) => { try { set({ itemTags: { ...get().itemTags, [itemId]: await api.assignItemTag(itemId, name) } }); await get().refreshTags(); } catch (e) { set({ error: String(e) }); } },
  removeItemTag: async (itemId, name) => { try { set({ itemTags: { ...get().itemTags, [itemId]: await api.removeItemTag(itemId, name) } }); await get().refreshTags(); } catch (e) { set({ error: String(e) }); } },

  setQuery: (query) => {
    set({ query });
    // 组合态期间先不查，等 compositionend 再一次性查
    if (get().composing) return;

    window.clearTimeout(refreshTimer);
    refreshTimer = window.setTimeout(() => {
      void get().refresh();
    }, 80);
  },

  setComposing: (composing) => {
    set({ composing });
    if (!composing) {
      void get().refresh();
    }
  },

  setFilter: (filter) => {
    set({ filter });
    void get().refresh();
  },

  cycleFilter: (dir) => {
    const idx = FILTER_ORDER.indexOf(get().filter);
    const len = FILTER_ORDER.length;
    const next = FILTER_ORDER[(((idx + dir) % len) + len) % len];
    if (!next) return;
    set({ filter: next });
    void get().refresh();
  },

  moveSelection: (delta) => {
    const { items, selectedItemId } = get();
    if (items.length === 0) {
      set({ selectedItemId: null, selectedIds: [] });
      return;
    }
    const idx = items.findIndex((i) => i.id === selectedItemId);
    const base = idx < 0 ? 0 : idx;
    const next = Math.min(items.length - 1, Math.max(0, base + delta));
    const target = items[next];
    if (target) {
      // 键盘移动是「重新定位光标」，顺带把批量选择收起 —— 否则上下移动之后
      // 屏幕上还留着一片高亮，用户不知道回车会粘哪一条。
      set({ selectedItemId: target.id, selectedIds: [target.id], selectionAnchorId: target.id });
    }
  },

  selectOnly: (id) => set({ selectedItemId: id, selectedIds: [id], selectionAnchorId: id }),

  toggleSelected: (id) => {
    const { selectedIds, selectedItemId } = get();
    const has = selectedIds.includes(id);
    const next = has ? selectedIds.filter((x) => x !== id) : [...selectedIds, id];
    // 取消掉的正好是光标项时，把光标挪到剩下的最后一个 —— 预览区不能空着
    const cursor = has && selectedItemId === id ? (next[next.length - 1] ?? null) : id;
    set({
      selectedIds: next,
      selectedItemId: cursor,
      // 锚点跟着走：Ctrl 点击之后接 Shift 点击，应该从最后点的那条开始扩选
      selectionAnchorId: id,
    });
  },

  selectRange: (id) => {
    const { items, selectionAnchorId, selectedItemId } = get();
    const anchor = selectionAnchorId ?? selectedItemId ?? id;
    const a = items.findIndex((i) => i.id === anchor);
    const b = items.findIndex((i) => i.id === id);
    if (a < 0 || b < 0) {
      get().selectOnly(id);
      return;
    }
    const [lo, hi] = a <= b ? [a, b] : [b, a];
    // 锚点不动，所以再 Shift 点一次更远的行是「重新扩选」而不是继续累加
    set({ selectedIds: items.slice(lo, hi + 1).map((i) => i.id), selectedItemId: id });
  },

  selectAllVisible: () => {
    const { items, selectedItemId } = get();
    if (items.length === 0) return;
    const ids = items.map((i) => i.id);
    const cursor = selectedItemId !== null && ids.includes(selectedItemId) ? selectedItemId : items[0]!.id;
    set({ selectedIds: ids, selectedItemId: cursor, selectionAnchorId: cursor });
  },

  clearMultiSelect: () => {
    const { selectedItemId } = get();
    set({ selectedIds: selectedItemId === null ? [] : [selectedItemId] });
  },

  assignGroupToSelection: async (groupId) => {
    const ids = get().selectedIds;
    if (ids.length === 0) return;
    try {
      // 逐条调用而不是新开一个批量命令：几十条的串行 IPC 完全可以接受，
      // 换来的是不用为「批量归类」再加一处四侧契约要维护。
      for (const id of ids) {
        await api.assignItemGroup(id, groupId);
      }
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deleteSelection: async () => {
    const ids = get().selectedIds;
    if (ids.length === 0) return;
    try {
      for (const id of ids) {
        await api.deleteItem(id);
      }
      // 先清空选择再刷新：刷新看到的是「这些 id 已经不在列表里」，
      // 不清的话它会退化成「选中第一条」，视觉上像跳了一下。
      set({ selectedIds: [], selectedItemId: null, selectionAnchorId: null });
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  pinItem: async (id) => {
    try {
      await api.togglePin(id);
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deleteItem: async (id) => {
    try {
      await api.deleteItem(id);
      // 删掉的正好是当前选中项才清空选中，否则保持不动
      if (get().selectedItemId === id) {
        set({ selectedItemId: null });
      }
      // 多选集合里也要摘掉，否则批量动作会对着一个已删除的 id 发请求
      set({ selectedIds: get().selectedIds.filter((x) => x !== id) });
      await get().refresh();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  removeSelected: async () => {
    const { selectedItemId } = get();
    if (selectedItemId === null) return;
    await get().deleteItem(selectedItemId);
  },

  togglePinSelected: async () => {
    const { selectedItemId } = get();
    if (selectedItemId === null) return;
    await get().pinItem(selectedItemId);
  },

  pasteSelected: async (asPlainText) => {
    const { selectedItemId } = get();
    if (selectedItemId === null) return;
    try {
      await api.pasteItem(selectedItemId, asPlainText);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  pasteItem: async (id, asPlainText) => {
    try {
      await api.pasteItem(id, asPlainText);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  editingId: null,
  editingDraft: "",
  setEditing: (id) => set({ editingId: id }),
  setEditingDraft: (text) => set({ editingDraft: text }),

  beginEditSelected: () => {
    const { selectedItemId, items } = get();
    if (selectedItemId === null) return;
    const item = items.find((i) => i.id === selectedItemId);
    if (!item) return;
    // 只有文本类（text/rich_text/link/color）能编辑，图片和文件没意义
    if (item.type === "image" || item.type === "files") return;
    set({
      editingId: selectedItemId,
      editingDraft: item.plain_text ?? item.content ?? "",
    });
  },

  pasteText: async (text, asPlainText) => {
    try {
      await api.pasteText(text, asPlainText);
      set({ editingId: null, editingDraft: "" });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  pasteSnippet: async (content) => {
    try {
      await api.pasteSnippet(content);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  clearQuery: () => {
    if (get().query.length === 0) return false;
    set({ query: "" });
    void get().refresh();
    return true;
  },

  reset: () => {
    // selectedGroupId 也必须清掉：分组过滤在界面上没有常驻指示位，
    // 不清的话唤起面板会带着上次选中的分组，列表看起来「数据全没了」。
    set({ query: "", selectedItemId: null, selectedIds: [], selectionAnchorId: null, selectedGroupId: null, error: null });
    void get().refresh();
  },
}));
