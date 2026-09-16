import type {
  AppInfo,
  AppSettings,
  FilterKind,
  Item,
  ItemType,
  Rule,
  RuleKind,
  Stats,
} from "../types";
import { splitCursorPlaceholder } from "./cursor";

const MINUTE = 60_000;
const BASE = Date.now();

const MOCK_THUMB =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAYAAADED76LAAAAE0lEQVR4nGMQWjrlPz7MMDIUAAA4rZKBu/1CPQAAAABJRU5ErkJggg==";

const SETTINGS_KEY = "plico.mock.settings";
const RULES_KEY = "plico.mock.rules";
const SNIPPETS_KEY = "plico.mock.snippets";
let mockSnippetSeq = 2;
const DEFAULT_SNIPPETS: import("../types").Snippet[] = [
  { id: 1, title: "会议开场", content: "大家好，感谢参加今天的会议。", tags: "会议", shortcut: null, created_at: BASE, updated_at: BASE },
  { id: 2, title: "代码块", content: "```ts\n// paste your code here\n```", tags: "代码", shortcut: "Ctrl+1", created_at: BASE, updated_at: BASE },
];
function readSnippets() { try { const raw = localStorage.getItem(SNIPPETS_KEY); if (raw) return JSON.parse(raw) as import("../types").Snippet[]; } catch {} return DEFAULT_SNIPPETS.map(s => ({ ...s })); }
function writeSnippets(rows: import("../types").Snippet[]) { localStorage.setItem(SNIPPETS_KEY, JSON.stringify(rows)); }
const GROUPS_KEY = "plico.mock.groups";
const DEFAULT_GROUPS: import("../types").Group[] = [
  { id: 1, name: "工作", color: "var(--pl-group-mint)", sort_order: 0 },
  { id: 2, name: "收藏", color: "var(--pl-group-sky)", sort_order: 1 },
];
function readGroups(): import("../types").Group[] {
  try {
    const raw = localStorage.getItem(GROUPS_KEY);
    if (raw) return JSON.parse(raw) as import("../types").Group[];
  } catch {}
  return DEFAULT_GROUPS.map((g) => ({ ...g }));
}
function writeGroups(rows: import("../types").Group[]) {
  localStorage.setItem(GROUPS_KEY, JSON.stringify(rows));
}
let groupSeq = readGroups().reduce((m, g) => Math.max(m, g.id), 0);
let tags: import("../types").Tag[] = [{ id: 1, name: "重要" }, { id: 2, name: "工作" }];
let itemTags: Record<number, number[]> = { 101: [1], 103: [2] };
let tagSeq = 2;

let seq = 100;
let panelPinned = false;

function make(partial: Partial<Item> & { type: ItemType }): Item {
  const id = ++seq;
  return {
    id,
    content: null,
    html_content: null,
    plain_text: null,
    image_path: null,
    thumb_path: null,
    content_hash: `mock-${id}`,
    source_app: null,
    source_title: null,
    group_id: null,
    pinned: false,
    created_at: BASE,
    last_copied_at: BASE,
    last_used_at: null,
    ...partial,
  };
}

let items: Item[] = [
  make({
    type: "text",
    content: "给张三的邮件草稿：关于 Q4 预算的回复，重点是研发投入要拆成人力与设备两块。",
    plain_text: "给张三的邮件草稿：关于 Q4 预算的回复，重点是研发投入要拆成人力与设备两块。",
    source_app: "OUTLOOK.EXE",
    source_title: "收件箱 - Outlook",
    pinned: true,
    last_copied_at: BASE - 4 * MINUTE,
  }),
  make({
    type: "image",
    content: "截图-2026-09-11 14.30.02.png",
    plain_text: "截图-2026-09-11 14.30.02.png",
    image_path: MOCK_THUMB,
    thumb_path: MOCK_THUMB,
    source_app: "SnippingTool.exe",
    source_title: "截图工具",
    pinned: true,
    last_copied_at: BASE - 6 * MINUTE,
  }),
  make({
    type: "rich_text",
    content:
      "Q4 版本排期\n1. 搜索性能：FTS5 索引重建\n2. 深色模式：调色板过一遍\n备注：周三前发评审材料",
    plain_text:
      "Q4 版本排期\n1. 搜索性能：FTS5 索引重建\n2. 深色模式：调色板过一遍\n备注：周三前发评审材料",
    html_content:
      '<h3>Q4 版本排期</h3><ol><li><b>搜索性能</b>：FTS5 <span style="color:#d97706">索引重建</span></li><li><b>深色模式</b>：<span style="color:#3B82F6">调色板</span>过一遍</li></ol><p><i>备注：周三前发评审材料</i></p>',
    source_app: "WINWORD.EXE",
    source_title: "Q4 排期.docx - Word",
    last_copied_at: BASE - 8 * MINUTE,
  }),
  make({
    type: "text",
    content:
      "小宋，Plico 的仓库我先建好了：github.com/plico/plico\n\n你 clone 下来跑一下 npm run tauri dev，有问题直接提 issue。",
    plain_text:
      "小宋，Plico 的仓库我先建好了：github.com/plico/plico\n\n你 clone 下来跑一下 npm run tauri dev，有问题直接提 issue。",
    source_app: "chrome.exe",
    source_title: "Plico · 功能规格说明书",
    last_copied_at: BASE - 11 * MINUTE,
  }),
  make({
    type: "link",
    content: "https://github.com/plico/plico",
    plain_text: "https://github.com/plico/plico",
    source_app: "chrome.exe",
    source_title: "GitHub",
    last_copied_at: BASE - 14 * MINUTE,
  }),
  make({
    type: "color",
    content: "#3B82F6",
    plain_text: "#3B82F6",
    source_app: "Figma.exe",
    source_title: "Plico 设计稿",
    last_copied_at: BASE - 16 * MINUTE,
  }),
  make({
    type: "files",
    content: '["D:\\\\工作\\\\Q4\\\\report.pdf","D:\\\\工作\\\\Q4\\\\data.xlsx","D:\\\\工作\\\\Q4\\\\notes.md"]',
    plain_text: "report.pdf data.xlsx notes.md",
    source_app: "explorer.exe",
    source_title: "Q4",
    last_copied_at: BASE - 18 * MINUTE,
  }),
  make({
    type: "text",
    content: "npm run tauri:dev",
    plain_text: "npm run tauri:dev",
    source_app: "WindowsTerminal.exe",
    source_title: "PowerShell",
    last_copied_at: BASE - 26 * MINUTE,
  }),
  make({
    type: "text",
    content:
      'SELECT id, "type", plain_text FROM items WHERE pinned = 1 ORDER BY last_copied_at DESC LIMIT 50;',
    plain_text:
      'SELECT id, "type", plain_text FROM items WHERE pinned = 1 ORDER BY last_copied_at DESC LIMIT 50;',
    source_app: "Code.exe",
    source_title: "db.rs - Plico - Visual Studio Code",
    last_copied_at: BASE - 33 * MINUTE,
  }),
  make({
    type: "text",
    content: "480 × 320",
    plain_text: "480 × 320",
    source_app: "Figma.exe",
    source_title: "Plico 设计稿",
    last_copied_at: BASE - 41 * MINUTE,
  }),
  make({
    type: "text",
    content: "C:\\Users\\Admin\\AppData\\Local\\Plico\\data\\plico.db",
    plain_text: "C:\\Users\\Admin\\AppData\\Local\\Plico\\data\\plico.db",
    source_app: "explorer.exe",
    source_title: "data",
    last_copied_at: BASE - 55 * MINUTE,
  }),
];

const DEFAULT_SETTINGS: AppSettings = {
  language: "system",
  autostart: false,
  hotkey: "Ctrl+Shift+V",
  plainHotkey: "Ctrl+Shift+Alt+V",
  maxItems: 1000,
  retentionDays: 30,
  imageQuotaMb: 500,
  hideOnBlur: true,
  hideDelayMs: 150,
  panelPosition: "cursor",
  panelX: null,
  panelY: null,
  panelW: null,
  panelH: null,
  previewCollapsed: false,
  vimMode: false,
  restoreClipboard: false,
};

const DEFAULT_RULES: Rule[] = [
  { id: 1, kind: "app", value: "1Password.exe", enabled: true },
  { id: 2, kind: "app", value: "KeePassXC.exe", enabled: true },
  { id: 3, kind: "content", value: "\\d{16}", enabled: false },
];

function readSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    return { ...DEFAULT_SETTINGS, ...(JSON.parse(raw) as Partial<AppSettings>) };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

function writeSettings(next: AppSettings) {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(next));
}

function freshRules(): Rule[] {
  return DEFAULT_RULES.map((r) => ({ ...r }));
}

function readRules(): Rule[] {
  try {
    const raw = localStorage.getItem(RULES_KEY);
    if (!raw) return freshRules();
    const parsed = JSON.parse(raw) as Rule[];
    return Array.isArray(parsed) ? parsed : freshRules();
  } catch {
    return freshRules();
  }
}

function writeRules(rules: Rule[]) {
  localStorage.setItem(RULES_KEY, JSON.stringify(rules));
}

const listeners = new Map<string, Set<() => void>>();

function emit(event: string) {
  listeners.get(event)?.forEach((fn) => fn());
}

export function mockListen(event: string, handler: () => void): Promise<() => void> {
  const set = listeners.get(event) ?? new Set();
  set.add(handler);
  listeners.set(event, set);
  return Promise.resolve(() => {
    set.delete(handler);
  });
}

if (typeof window !== "undefined") {
  window.addEventListener("storage", (e) => {
    if (e.key === SETTINGS_KEY) emit("plico://settings-changed");
  });
}

function stats(): Stats {
  return {
    total: items.length,
    pinned: items.filter((i) => i.pinned).length,
  };
}

function normalizeShortcut(spec: string): string {
  return spec
    .split("+")
    .map((part) => part.trim().toLowerCase())
    .filter(Boolean)
    .join("+");
}

function checkMockShortcut(
  rows: import("../types").Snippet[],
  raw: string | null,
  selfId: number | null,
): string | null {
  const spec = (raw ?? "").trim();
  if (!spec) return null;

  const parts = spec.split("+").map((p) => p.trim());
  if (parts.some((p) => !/^[A-Za-z0-9]+$/.test(p))) {
    throw new Error(`无法识别的快捷键「${spec}」`);
  }

  const key = normalizeShortcut(spec);
  const settings = readSettings();
  const reserved: Array<[string, string]> = [
    ["唤起面板", settings.hotkey],
    ["粘贴为纯文本", settings.plainHotkey],
  ];
  for (const [label, other] of reserved) {
    if (normalizeShortcut(other) === key) {
      throw new Error(`快捷键 ${spec} 已被「${label}」占用`);
    }
  }
  for (const s of rows) {
    if (s.id === selfId) continue;
    const other = (s.shortcut ?? "").trim();
    if (other && normalizeShortcut(other) === key) {
      throw new Error(`快捷键 ${spec} 已被片段「${s.title}」占用`);
    }
  }
  return spec;
}

const CODE_PROFILES: Array<{ lang: string; ci: boolean; signals: Array<[string, number]> }> = [
  { lang: "rust", ci: false, signals: [["fn ", 3], ["impl ", 3], ["pub ", 2], ["let mut ", 3], ["&str", 3], ["&mut ", 3], ["println!", 4], ["#[", 3], ["-> ", 2], ["crate::", 3], ["unwrap()", 3], ["Option<", 3], ["Result<", 3], ["vec![", 3], ["match ", 2], ["::", 1], ["use ", 1]] },
  { lang: "typescript", ci: false, signals: [[": string", 4], [": number", 4], [": boolean", 4], [": void", 4], ["interface ", 4], ["implements ", 3], ["readonly ", 3], ["export type ", 4], [" as const", 3], ["<T>", 3], ["?: ", 2], ["): ", 2], ["enum ", 2], ["| null", 2], ["| undefined", 2]] },
  { lang: "javascript", ci: false, signals: [["console.log", 4], ["require(", 4], ["module.exports", 4], ["function ", 3], ["var ", 3], ["document.", 3], ["JSON.", 3], ["const ", 2], ["=> ", 2], ["async ", 2], ["await ", 2], ["window.", 2], ["let ", 1], ["export ", 1], ["import ", 1]] },
  { lang: "python", ci: false, signals: [["def ", 4], ["elif ", 4], ["self.", 4], ["__init__", 4], ["print(", 3], ["lambda ", 3], ["try:", 3], ["except ", 3], ['f"', 3], ["None", 2], ["True", 2], ["False", 2], ["range(", 2], ["import ", 1], ["from ", 1]] },
  { lang: "css", ci: true, signals: [["display:", 4], ["font-size:", 4], ["border-radius:", 4], ["grid-template", 4], ["!important", 4], ["@media", 4], ["background:", 3], ["color:", 3], ["margin:", 3], ["padding:", 3], ["var(--", 3], ["flex", 2], ["rem;", 2], ["px;", 1]] },
  { lang: "sql", ci: true, signals: [["create table", 5], ["insert into ", 5], ["delete from ", 5], ["select ", 4], ["group by", 4], ["order by", 4], ["primary key", 4], ["where ", 3], ["join ", 3], ["values (", 3], ["update ", 3], ["from ", 2], ["limit ", 2]] },
  { lang: "shell", ci: false, signals: [["echo ", 4], ["sudo ", 4], ["apt-get", 4], ["chmod ", 4], ["$(", 3], ["${", 3], ["mkdir ", 3], ["&& ", 2], ["|| ", 2], ["export ", 2], ["npm ", 2], ["git ", 2], ["then\n", 3], ["fi\n", 4]] },
  { lang: "html", ci: true, signals: [["<!doctype", 5], ["<div", 4], ["<span", 4], ["<p>", 4], ["<img", 4], ["<head", 4], ["<body", 4], ["<meta", 4], ["class=", 3], ["href=", 3], ["<a ", 3], ["<ul", 3], ["<li", 3], ["</", 2]] },
  { lang: "json", ci: false, signals: [['": {', 4], ['": [', 4], ['": true', 4], ['": false', 4], ['": null', 4], ['": "', 3], ['"},\n', 3], ["[\n  {", 3], ['": 0', 2]] },
];

function stripCodeNoise(src: string): string {
  return src
    .replace(/\/\*[\s\S]*?\*\//g, " ")
    .replace(/\/\/[^\n]*/g, " ")
    .replace(/"(?:\\.|[^"\\\n])*"/g, " ")
    .replace(/`(?:\\.|[^`\\])*`/g, " ")
    .replace(/^[ \t]*#[^\n]*/gm, " ");
}

function detectCodeMock(raw: string): string | null {
  const text = raw.trim();
  if (!text) return null;

  if (text.startsWith("#!")) return "shell";
  if ((text.startsWith("{") || text.startsWith("["))) {
    try {
      JSON.parse(text);
      return "json";
    } catch {
    }
  }
  if (text.startsWith("<") && (text.match(/</g)?.length ?? 0) >= 3 && (text.includes("</") || text.includes("/>"))) {
    return "html";
  }

  if (text.length < 30) return null;

  const skeleton = stripCodeNoise(text);
  const lowered = skeleton.toLowerCase();
  const threshold = text.split("\n").length >= 2 ? 3 : 8;
  let best: string | null = null;
  let bestScore = 0;
  for (const p of CODE_PROFILES) {
    const hay = p.ci ? lowered : skeleton;
    const score = p.signals.reduce((sum, [needle, weight]) => (hay.includes(needle) ? sum + weight : sum), 0);
    if (score < threshold) continue;
    if (score > bestScore) {
      best = p.lang;
      bestScore = score;
    }
  }
  return best;
}

function tagStats(): import("../types").TagStat[] {
  return tags
    .map((tag) => ({
      id: tag.id,
      name: tag.name,
      count: Object.values(itemTags).filter((ids) => ids.includes(tag.id)).length,
    }))
    .sort((x, y) => x.name.localeCompare(y.name));
}

function matchesQuery(item: Item, q: string): boolean {
  const tagMatch = q.match(/(?:^|\s)tag:([^\s]+)/i);
  if (tagMatch) {
    const wanted = tagMatch[1]?.toLowerCase() ?? "";
    const ids = itemTags[item.id] ?? [];
    if (!ids.some(id => tags.find(t => t.id === id)?.name.toLowerCase() === wanted)) return false;
    q = q.replace(tagMatch[0], " ").trim();
  }
  const needle = q.toLowerCase();
  return [item.plain_text, item.content, item.source_app, item.source_title]
    .filter((v): v is string => typeof v === "string")
    .some((v) => v.toLowerCase().includes(needle));
}

function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, Math.round(v)));
}

function applyPatch(current: AppSettings, patch: Partial<AppSettings>): AppSettings {
  const next: AppSettings = { ...current };
  const target = next as unknown as Record<string, unknown>;
  for (const key of Object.keys(current) as (keyof AppSettings)[]) {
    if (key in patch && patch[key] !== undefined) {
      target[key] = patch[key];
    }
  }

  next.maxItems = clamp(next.maxItems, 100, 10_000);
  next.retentionDays = clamp(next.retentionDays, 1, 365);
  next.imageQuotaMb = clamp(next.imageQuotaMb, 50, 10_240);
  next.hideDelayMs = clamp(next.hideDelayMs, 0, 5_000);
  if (next.panelW !== null) next.panelW = clamp(next.panelW, 320, 3_000);
  if (next.panelH !== null) next.panelH = clamp(next.panelH, 200, 2_000);
  if (!next.hotkey.trim()) next.hotkey = DEFAULT_SETTINGS.hotkey;
  if (!next.language.trim()) next.language = "system";

  return next;
}

let ruleSeq = 100;

export async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await dispatchMock<T>(cmd, args);
  } catch (e) {
    throw e instanceof Error ? e.message : e;
  }
}

async function dispatchMock<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((r) => setTimeout(r, 30));

  const a = args ?? {};

  switch (cmd) {
    case "list_items": {
      const query = (a.query as string | null) ?? "";
      const kind = (a.kind as FilterKind | null) ?? null;
      const groupId = (a.groupId as number | null) ?? null;
      const limit = (a.limit as number) ?? 500;

      const filtered = items
        .filter((i) => (query ? matchesQuery(i, query) : true))
        .filter((i) => (kind ? i.type === kind : true))
        .filter((i) => (groupId === null ? true : i.group_id === groupId || i.pinned))
        .sort((x, y) => {
          if (x.pinned !== y.pinned) return x.pinned ? -1 : 1;
          return y.last_copied_at - x.last_copied_at;
        })
        .slice(0, limit);

      return filtered as T;
    }

    case "list_tags": return tags as T;
    case "get_item_tags": {
      const ids = itemTags[a.itemId as number] ?? [];
      return tags.filter((tag) => ids.includes(tag.id)) as T;
    }
    case "assign_item_tag": {
      const itemId = a.itemId as number;
      const name = String(a.name ?? "").trim();
      if (!name) throw new Error("标签不能为空");
      let tag = tags.find((x) => x.name === name);
      if (!tag) { tag = { id: ++tagSeq, name }; tags = [...tags, tag]; }
      itemTags[itemId] = [...new Set([...(itemTags[itemId] ?? []), tag.id])];
      return tags.filter((x) => itemTags[itemId]?.includes(x.id)) as T;
    }
    case "remove_item_tag": {
      const itemId = a.itemId as number;
      const tag = tags.find((x) => x.name === String(a.name ?? "").trim());
      if (tag) itemTags[itemId] = (itemTags[itemId] ?? []).filter((id) => id !== tag.id);
      return tags.filter((x) => itemTags[itemId]?.includes(x.id)) as T;
    }

    case "list_tag_stats":
      return tagStats() as T;

    case "rename_tag": {
      const name = String(a.name ?? "").trim();
      if (!name) throw new Error("标签名不能为空");
      const tag = tags.find((x) => x.id === a.id);
      if (!tag) throw new Error(`找不到标签：${String(a.id)}`);
      if (tags.some((x) => x.name === name && x.id !== tag.id)) {
        throw new Error(`已存在同名标签「${name}」`);
      }
      tag.name = name;
      return tagStats() as T;
    }

    case "merge_tags": {
      const from = a.from as number;
      const to = a.to as number;
      if (from === to) return tagStats() as T;
      if (!tags.some((x) => x.id === from) || !tags.some((x) => x.id === to)) {
        throw new Error("找不到要合并的标签");
      }
      for (const [key, ids] of Object.entries(itemTags)) {
        if (!ids.includes(from)) continue;
        itemTags[Number(key)] = [...new Set([...ids.filter((id) => id !== from), to])];
      }
      tags = tags.filter((x) => x.id !== from);
      return tagStats() as T;
    }

    case "delete_tag": {
      const id = a.id as number;
      if (!tags.some((x) => x.id === id)) throw new Error(`找不到标签：${String(id)}`);
      tags = tags.filter((x) => x.id !== id);
      for (const [key, ids] of Object.entries(itemTags)) {
        itemTags[Number(key)] = ids.filter((x) => x !== id);
      }
      return tagStats() as T;
    }

    case "list_groups": return readGroups() as T;
    case "create_group": {
      const groups = readGroups();
      const name = String(a.name ?? "").trim();
      if (!name) throw new Error("分组名称不能为空");
      if (groups.some((g) => g.name === name)) throw new Error(`已存在同名分组「${name}」`);
      const color = (a.color as string | null) ?? null;
      const next = [...groups, { id: ++groupSeq, name, color, sort_order: groups.length }];
      writeGroups(next);
      return next as T;
    }
    case "rename_group": {
      const groups = readGroups();
      const g = groups.find((x) => x.id === a.id);
      if (!g) throw new Error(`找不到分组 ${a.id}`);
      const name = String(a.name ?? g.name).trim();
      if (!name) throw new Error("分组名称不能为空");
      if (groups.some((x) => x.name === name && x.id !== g.id)) throw new Error(`已存在同名分组「${name}」`);
      g.name = name;
      if (a.color != null) g.color = String(a.color);
      writeGroups(groups);
      return groups as T;
    }
    case "delete_group": {
      const groups = readGroups();
      if (!groups.some((g) => g.id === a.id)) throw new Error(`找不到分组 ${a.id}`);
      writeGroups(groups.filter((g) => g.id !== a.id));
      items = items.map((i) => i.group_id === a.id ? { ...i, group_id: null } : i);
      return readGroups() as T;
    }
    case "reorder_groups": {
      const groups = readGroups();
      const ids = (a.ids as number[]) ?? [];
      if (new Set(ids).size !== ids.length) throw new Error("分组顺序里有重复项");
      const existing = new Set(groups.map((g) => g.id));
      if (ids.length !== existing.size || ids.some((id) => !existing.has(id))) {
        throw new Error("分组列表已变化，请重试");
      }
      const byId = new Map(groups.map((g) => [g.id, g]));
      const next = ids.map((id, index) => ({ ...byId.get(id)!, sort_order: index }));
      writeGroups(next);
      return next as T;
    }
    case "assign_item_group": { const item = items.find((i) => i.id === a.itemId); if (item) item.group_id = (a.groupId as number | null) ?? null; return undefined as T; }

    case "list_snippets": return readSnippets() as T;
    case "create_snippet": {
      const rows = readSnippets();
      const now = Date.now();
      const shortcut = checkMockShortcut(rows, (a.shortcut as string | null) ?? null, null);
      const s = { id: ++mockSnippetSeq, title: String(a.title ?? "").trim(), content: String(a.content ?? ""), tags: (a.tags as string | null) ?? null, shortcut, created_at: now, updated_at: now };
      rows.unshift(s); writeSnippets(rows); return s as T;
    }
    case "update_snippet": {
      const rows = readSnippets();
      const s = rows.find(x => x.id === a.id);
      if (!s) throw new Error("找不到片段");
      const shortcut = checkMockShortcut(rows, (a.shortcut as string | null) ?? null, a.id as number);
      Object.assign(s, { title: String(a.title ?? "").trim(), content: String(a.content ?? ""), tags: (a.tags as string | null) ?? null, shortcut, updated_at: Date.now() });
      writeSnippets(rows); return undefined as T;
    }
    case "delete_snippet": { writeSnippets(readSnippets().filter(x => x.id !== a.id)); return undefined as T; }

    case "get_stats":
      return stats() as T;

    case "get_item": {
      const found = items.find((i) => i.id === (a.id as number));
      if (!found) throw new Error(`找不到条目：${String(a.id)}`);
      return found as T;
    }

    case "delete_item": {
      items = items.filter((i) => i.id !== (a.id as number));
      emit("plico://items-changed");
      return undefined as T;
    }

    case "toggle_pin": {
      const target = items.find((i) => i.id === (a.id as number));
      if (!target) throw new Error(`找不到条目：${String(a.id)}`);
      target.pinned = !target.pinned;
      emit("plico://items-changed");
      return target as T;
    }

    case "clear_history": {
      const keepPinned = (a.keepPinned as boolean) ?? true;
      const before = items.length;
      items = keepPinned ? items.filter((i) => i.pinned) : [];
      emit("plico://items-changed");
      return (before - items.length) as T;
    }

    case "paste_item": {
      const target = items.find((i) => i.id === (a.id as number));
      if (!target) throw new Error(`找不到条目：${String(a.id)}`);
      target.last_used_at = Date.now();
      const mode = a.asPlainText
        ? "（纯文本）"
        : target.html_content
          ? "（富文本 HTML+纯文本）"
          : "";
      console.info(`[mock] 模拟粘贴${mode}：`, target.plain_text ?? target.content);
      return undefined as T;
    }

    case "paste_text": {
      console.info("[mock] 模拟粘贴文本：", String(a.text ?? "").slice(0, 60));
      return undefined as T;
    }

    case "paste_text_with_cursor": {
      const { text, leftMoves } = splitCursorPlaceholder(String(a.text ?? ""));
      console.info("[mock] 模拟粘贴片段：", { 文本: text.slice(0, 60), 左移次数: leftMoves });
      return undefined as T;
    }

    case "copy_text": {
      console.info("[mock] 复制文本：", String(a.text ?? "").slice(0, 80));
      return undefined as T;
    }

    case "detect_code": {
      return detectCodeMock(String(a.text ?? "")) as T;
    }

    case "hide_panel":
      console.info("[mock] 模拟隐藏面板");
      return undefined as T;

    case "toggle_panel_pin": {
      panelPinned = !panelPinned;
      console.info(`[mock] 面板钉住切换：${panelPinned ? "已钉住" : "未钉住"}`);
      return panelPinned as T;
    }

    case "is_panel_pinned":
      return panelPinned as T;

    case "get_settings":
      return readSettings() as T;

    case "update_settings": {
      const next = applyPatch(readSettings(), (a.patch ?? {}) as Partial<AppSettings>);
      writeSettings(next);
      emit("plico://settings-changed");
      return next as T;
    }

    case "list_rules":
      return readRules() as T;

    case "add_rule": {
      const value = String(a.value ?? "").trim();
      if (!value) throw new Error("规则内容不能为空");
      if (a.kind === "content") {
        try {
          new RegExp(value);
        } catch (e) {
          throw new Error(`正则表达式无效：${String(e)}`);
        }
      }
      const rules = readRules();
      rules.push({
        id: ++ruleSeq + 1000,
        kind: a.kind as RuleKind,
        value,
        enabled: true,
      });
      writeRules(rules);
      return rules as T;
    }

    case "delete_rule": {
      const rules = readRules().filter((r) => r.id !== (a.id as number));
      writeRules(rules);
      return rules as T;
    }

    case "set_rule_enabled": {
      const rules = readRules().map((r) =>
        r.id === (a.id as number) ? { ...r, enabled: a.enabled as boolean } : r,
      );
      writeRules(rules);
      return rules as T;
    }

    case "open_settings": {
      window.open(`${location.pathname}?window=settings`, "_blank");
      return undefined as T;
    }

    case "get_app_info": {
      const base = "C:\\Users\\Admin\\AppData\\Local\\Plico\\data";
      const info: AppInfo = {
        version: "0.1.0-mock",
        dataDir: base,
        dbPath: `${base}\\plico.db`,
      };
      return info as T;
    }

    default:
      throw new Error(`[mock] 未实现的命令：${cmd}`);
  }
}
