import { invoke } from "./lib/bridge";
import type {
  AppInfo,
  AppSettings,
  FilterKind,
  Item,
  Rule,
  RuleKind,
  Stats,
  Group,
  Tag,
  TagStat,
} from "./types";

export function listTags(): Promise<Tag[]> { return invoke<Tag[]>("list_tags"); }
export function assignItemTag(itemId:number, name:string): Promise<Tag[]> { return invoke<Tag[]>("assign_item_tag", { itemId, name }); }
export function removeItemTag(itemId:number, name:string): Promise<Tag[]> { return invoke<Tag[]>("remove_item_tag", { itemId, name }); }
export function getItemTags(itemId:number): Promise<Tag[]> { return invoke<Tag[]>("get_item_tags", { itemId }); }

// ---- F17 标签管理 ----
// 这四个都返回变更后的完整统计列表：标签页是「一屏看全」的形态，
// 每次改完拿后端的权威数据覆盖本地，比前端自己打补丁可靠。

export function listTagStats(): Promise<TagStat[]> { return invoke<TagStat[]>("list_tag_stats"); }
export function renameTag(id: number, name: string): Promise<TagStat[]> { return invoke<TagStat[]>("rename_tag", { id, name }); }
/** 把 `from` 的条目归属整体并到 `to`，然后删掉 `from`。 */
export function mergeTags(from: number, to: number): Promise<TagStat[]> { return invoke<TagStat[]>("merge_tags", { from, to }); }
export function deleteTag(id: number): Promise<TagStat[]> { return invoke<TagStat[]>("delete_tag", { id }); }

// ---- F16 分组管理 ----
// 三个写操作都返回变更后的完整分组列表：新建要知道新条目的 id，删除要清掉
// 当前的筛选选中项。和 F17 标签页同一立场 —— 拿后端的权威数据覆盖本地。

export function listGroups(): Promise<Group[]> { return invoke<Group[]>("list_groups"); }
export function createGroup(name: string, color?: string | null): Promise<Group[]> { return invoke<Group[]>("create_group", { name, color: color ?? null }); }
/** `color` 传 `undefined` 表示只改名、不动颜色。 */
export function renameGroup(id: number, name: string, color?: string | null): Promise<Group[]> { return invoke<Group[]>("rename_group", { id, name, color: color ?? null }); }
export function deleteGroup(id: number): Promise<Group[]> { return invoke<Group[]>("delete_group", { id }); }
/**
 * 按**完整 id 顺序**重写分组排序。
 *
 * 传的是整份顺序而不是「把 X 上移一位」：拖完/点完上下移，前端手里本来就有一份
 * 完整列表，整份提交最省事，也不会出现两个增量指令互相抵消。后端会校验 id 集合
 * 与库中完全一致，拿着过期列表提交会报错 —— 那时重取一次列表即可。
 */
export function reorderGroups(ids: number[]): Promise<Group[]> { return invoke<Group[]>("reorder_groups", { ids }); }
export function assignItemGroup(itemId: number, groupId: number | null): Promise<void> { return invoke<void>("assign_item_group", { itemId, groupId }); }
export function listSnippets(): Promise<import("./types").Snippet[]> { return invoke("list_snippets"); }
export function createSnippet(title:string, content:string, tags?:string|null, shortcut?:string|null) { return invoke<import("./types").Snippet>("create_snippet", {title,content,tags:tags??null,shortcut:shortcut??null}); }
export function updateSnippet(id:number,title:string,content:string,tags?:string|null,shortcut?:string|null) { return invoke<void>("update_snippet", {id,title,content,tags:tags??null,shortcut:shortcut??null}); }
export function deleteSnippet(id:number) { return invoke<void>("delete_snippet", {id}); }
export function listItems(
  query?: string,
  kind?: FilterKind,
  limit = 500,
): Promise<Item[]> {
  return invoke<Item[]>("list_items", {
    query: query && query.length > 0 ? query : null,
    kind: !kind || kind === "all" ? null : kind,
    limit,
  });
}

export function getItem(id: number): Promise<Item> {
  return invoke<Item>("get_item", { id });
}

export function deleteItem(id: number): Promise<void> {
  return invoke<void>("delete_item", { id });
}

export function togglePin(id: number): Promise<Item> {
  return invoke<Item>("toggle_pin", { id });
}

export function clearHistory(keepPinned = true): Promise<number> {
  return invoke<number>("clear_history", { keepPinned });
}

export function getStats(): Promise<Stats> {
  return invoke<Stats>("get_stats");
}

export function pasteItem(id: number, asPlainText?: boolean): Promise<void> {
  return invoke<void>("paste_item", { id, asPlainText: asPlainText ?? null });
}

/** F19：直接粘一段文本（编辑后的内容，不入库为已有条目）。 */
export function pasteText(text: string, asPlainText?: boolean): Promise<void> {
  return invoke<void>("paste_text", { text, asPlainText: asPlainText ?? null });
}

/** F18：粘片段内容，`{{cursor}}` 占位符由后端解析，粘贴后光标停在占位符处。 */
export function pasteSnippet(text: string): Promise<void> {
  return invoke<void>("paste_text_with_cursor", { text });
}

/** F20：在当前系统剪贴板写一段文本（Markdown 链接等轻量动作）。 */
export function copyText(text: string): Promise<void> {
  return invoke<void>("copy_text", { text });
}

/** F20：猜一段文本像哪种代码语言。不像代码时返回 null。 */
export function detectCode(text: string): Promise<string | null> {
  return invoke<string | null>("detect_code", { text });
}

export function hidePanel(): Promise<void> {
  return invoke<void>("hide_panel");
}

/** 面板「钉住桌面」：切换钉住状态，返回新值（true=已钉住）。 */
export function togglePanelPin(): Promise<boolean> {
  return invoke<boolean>("toggle_panel_pin");
}

/** 查询面板当前是否钉住。 */
export function isPanelPinned(): Promise<boolean> {
  return invoke<boolean>("is_panel_pinned");
}

// ---------------- 设置（F22）----------------

export function getSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_settings");
}

/**
 * 只传要改的字段。返回值是后端落库后的**权威设置**（越界值已被钳制），
 * 调用方应拿它覆盖本地状态。
 */
export function updateSettings(patch: Partial<AppSettings>): Promise<AppSettings> {
  return invoke<AppSettings>("update_settings", { patch });
}

// ---------------- 隐私规则（F10）----------------

export function listRules(): Promise<Rule[]> {
  return invoke<Rule[]>("list_rules");
}

export function addRule(kind: RuleKind, value: string): Promise<Rule[]> {
  return invoke<Rule[]>("add_rule", { kind, value });
}

export function deleteRule(id: number): Promise<Rule[]> {
  return invoke<Rule[]>("delete_rule", { id });
}

export function setRuleEnabled(id: number, enabled: boolean): Promise<Rule[]> {
  return invoke<Rule[]>("set_rule_enabled", { id, enabled });
}

// ---------------- 窗口与系统入口 ----------------

export function openSettings(): Promise<void> {
  return invoke<void>("open_settings");
}

/** 关于页用的信息：版本 + 数据目录/数据库路径（只读展示，不可更改）。 */
export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}
