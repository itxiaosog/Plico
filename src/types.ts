import { t } from "./lib/i18n";

export type ItemType = "text" | "rich_text" | "image" | "files" | "link" | "color";

export interface Item {
  id: number;
  type: ItemType;
  content: string | null;
  html_content: string | null;
  plain_text: string | null;
  image_path: string | null;
  thumb_path: string | null;
  content_hash: string;
  source_app: string | null;
  source_title: string | null;
  group_id: number | null;
  pinned: boolean;
  created_at: number;
  last_copied_at: number;
  last_used_at: number | null;
}

export interface Snippet { id:number; title:string; content:string; tags:string|null; shortcut:string|null; created_at:number; updated_at:number; }

export interface Group { id: number; name: string; color: string | null; sort_order: number; }
export interface Tag { id: number; name: string }

/**
 * 标签 + 使用计数（F17）。标签页要靠 `count` 决定「这个标签还留着吗」，
 * 所以哪怕计数为 0 的标签也会出现在列表里。
 */
export interface TagStat { id: number; name: string; count: number }

export interface Stats {
  total: number;
  pinned: number;
}

/** 类型筛选。`all` 是内置的"不筛选"，其余与 ItemType 对齐（F7）。 */
export type FilterKind = "all" | ItemType;

export const FILTER_ORDER: FilterKind[] = [
  "all",
  "text",
  "image",
  "files",
  "link",
  "color",
];

/**
 * 筛选 / 类型的展示名。
 * 文案集中在 `lib/i18n.ts`，这里改成函数形式取词，语言切换后才跟得上。
 * `filterLabel(k)` / `typeLabel(t)` 代替原来的 `FILTER_LABEL[k]` / `TYPE_LABEL[t]`。
 */
export function filterLabel(kind: FilterKind): string {
  const keys = {
    all: "filterAll",
    text: "filterText",
    rich_text: "filterText",
    image: "filterImage",
    files: "filterFiles",
    link: "filterLink",
    color: "filterColor",
  } as const;
  return t(keys[kind]);
}

export function typeLabel(kind: ItemType): string {
  const keys = {
    text: "typeText",
    rich_text: "typeRichText",
    image: "typeImage",
    files: "typeFiles",
    link: "typeLink",
    color: "typeColor",
  } as const;
  return t(keys[kind]);
}

// ---------------- 设置（F22）----------------

/** 面板位置模式（F5）。 */
export type PanelPosition = "cursor" | "remember";

/**
 * 与 Rust 侧 `settings::AppSettings` 一一对应。
 * Rust 用 `#[serde(rename_all = "camelCase")]`，所以这里全用驼峰。
 */
export interface AppSettings {
  /** system / zh / en。`LangPref` 的存储值，解析在 lib/lang.ts。 */
  language: string;
  autostart: boolean;
  hotkey: string;
  /** 「粘贴为纯文本」的全局热键（F15）。 */
  plainHotkey: string;

  maxItems: number;
  retentionDays: number;
  imageQuotaMb: number;

  hideOnBlur: boolean;
  hideDelayMs: number;
  panelPosition: PanelPosition;
  /** 面板记忆坐标（物理像素）。 */
  panelX: number | null;
  panelY: number | null;
  /** 面板记忆尺寸（逻辑像素）。null = 没记忆过，用窗口默认值。 */
  panelW: number | null;
  panelH: number | null;
  /** 预览区折叠（F5）。折叠后列表区撑满窗口宽度。 */
  previewCollapsed: boolean;
  /** 面板内用 J / K 移动选中（F5，默认关）。 */
  vimMode: boolean;

  /** F21，2.0 才生效。 */
  restoreClipboard: boolean;
}

/** 隐私规则类型（F10）：按进程名排除 / 按内容正则排除。 */
export type RuleKind = "app" | "content";

export interface Rule {
  id: number;
  kind: RuleKind;
  value: string;
  enabled: boolean;
}

/** 关于页（F22）需要的信息。 */
export interface AppInfo {
  version: string;
  dataDir: string;
  dbPath: string;
}
