import { create } from "zustand";

import * as api from "../api";
import { FILTER_ORDER, type FilterKind, type Item, type Stats } from "../types";

interface ClipboardState {
  items: Item[];
  snippets: import("../types").Snippet[];
  query: string;
  filter: FilterKind;
  selectedItemId: number | null;
  selectedIds: number[];
  selectionAnchorId: number | null;
  selectedSnippetId: number | null;
  stats: Stats;
  loading: boolean;
  composing: boolean;
  error: string | null;
  groups: import("../types").Group[];
  selectedGroupId: number | null;

  refresh: () => Promise<void>;
  refreshSnippets: () => Promise<void>;
  refreshGroups: () => Promise<void>;
  setSelectedGroup: (id: number | null) => void;
  createGroup: (name: string, color?: string | null) => Promise<void>;
  renameGroup: (id: number, name: string, color?: string | null) => Promise<void>;
  deleteGroup: (id: number) => Promise<void>;
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
  selectOnly: (id: number) => void;
  toggleSelected: (id: number) => void;
  selectRange: (id: number) => void;
  selectAllVisible: () => void;
  clearMultiSelect: () => void;
  assignGroupToSelection: (groupId: number | null) => Promise<void>;
  deleteSelection: () => Promise<void>;
  pinItem: (id: number) => Promise<void>;
  deleteItem: (id: number) => Promise<void>;
  removeSelected: () => Promise<void>;
  togglePinSelected: () => Promise<void>;
  pasteSelected: (asPlainText?: boolean) => Promise<void>;
  pasteItem: (id: number, asPlainText?: boolean) => Promise<void>;
  pasteText: (text: string, asPlainText?: boolean) => Promise<void>;
  pasteSnippet: (content: string) => Promise<void>;
  editingId: number | null;
  editingDraft: string;
  setEditing: (id: number | null) => void;
  setEditingDraft: (text: string) => void;
  beginEditSelected: () => void;
  clearQuery: () => boolean;
  reset: () => void;
}

let refreshTimer: number | undefined;
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

      if (seq !== requestSeq) return;

      const { selectedItemId, selectedIds } = get();
      const visibleIds = new Set(visibleItems.map((i) => i.id));
      const cursorKept = selectedItemId !== null && visibleIds.has(selectedItemId);

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
      set({ selectedItemId: target.id, selectedIds: [target.id], selectionAnchorId: target.id });
    }
  },

  selectOnly: (id) => set({ selectedItemId: id, selectedIds: [id], selectionAnchorId: id }),

  toggleSelected: (id) => {
    const { selectedIds, selectedItemId } = get();
    const has = selectedIds.includes(id);
    const next = has ? selectedIds.filter((x) => x !== id) : [...selectedIds, id];
    const cursor = has && selectedItemId === id ? (next[next.length - 1] ?? null) : id;
    set({
      selectedIds: next,
      selectedItemId: cursor,
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
      if (get().selectedItemId === id) {
        set({ selectedItemId: null });
      }
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
    set({ query: "", selectedItemId: null, selectedIds: [], selectionAnchorId: null, selectedGroupId: null, error: null });
    void get().refresh();
  },
}));
