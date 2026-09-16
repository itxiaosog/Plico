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

export interface TagStat { id: number; name: string; count: number }

export interface Stats {
  total: number;
  pinned: number;
}

export type FilterKind = "all" | ItemType;

export const FILTER_ORDER: FilterKind[] = [
  "all",
  "text",
  "image",
  "files",
  "link",
  "color",
];

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

export type PanelPosition = "cursor" | "remember";

export interface AppSettings {
  language: string;
  autostart: boolean;
  hotkey: string;
  plainHotkey: string;

  maxItems: number;
  retentionDays: number;
  imageQuotaMb: number;

  hideOnBlur: boolean;
  hideDelayMs: number;
  panelPosition: PanelPosition;
  panelX: number | null;
  panelY: number | null;
  panelW: number | null;
  panelH: number | null;
  previewCollapsed: boolean;
  vimMode: boolean;

  restoreClipboard: boolean;
}

export type RuleKind = "app" | "content";

export interface Rule {
  id: number;
  kind: RuleKind;
  value: string;
  enabled: boolean;
}

export interface AppInfo {
  version: string;
  dataDir: string;
  dbPath: string;
}
