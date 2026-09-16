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

export function listTagStats(): Promise<TagStat[]> { return invoke<TagStat[]>("list_tag_stats"); }
export function renameTag(id: number, name: string): Promise<TagStat[]> { return invoke<TagStat[]>("rename_tag", { id, name }); }
export function mergeTags(from: number, to: number): Promise<TagStat[]> { return invoke<TagStat[]>("merge_tags", { from, to }); }
export function deleteTag(id: number): Promise<TagStat[]> { return invoke<TagStat[]>("delete_tag", { id }); }

export function listGroups(): Promise<Group[]> { return invoke<Group[]>("list_groups"); }
export function createGroup(name: string, color?: string | null): Promise<Group[]> { return invoke<Group[]>("create_group", { name, color: color ?? null }); }
export function renameGroup(id: number, name: string, color?: string | null): Promise<Group[]> { return invoke<Group[]>("rename_group", { id, name, color: color ?? null }); }
export function deleteGroup(id: number): Promise<Group[]> { return invoke<Group[]>("delete_group", { id }); }
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

export function pasteText(text: string, asPlainText?: boolean): Promise<void> {
  return invoke<void>("paste_text", { text, asPlainText: asPlainText ?? null });
}

export function pasteSnippet(text: string): Promise<void> {
  return invoke<void>("paste_text_with_cursor", { text });
}

export function copyText(text: string): Promise<void> {
  return invoke<void>("copy_text", { text });
}

export function detectCode(text: string): Promise<string | null> {
  return invoke<string | null>("detect_code", { text });
}

export function hidePanel(): Promise<void> {
  return invoke<void>("hide_panel");
}

export function togglePanelPin(): Promise<boolean> {
  return invoke<boolean>("toggle_panel_pin");
}

export function isPanelPinned(): Promise<boolean> {
  return invoke<boolean>("is_panel_pinned");
}

export function getSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_settings");
}

export function updateSettings(patch: Partial<AppSettings>): Promise<AppSettings> {
  return invoke<AppSettings>("update_settings", { patch });
}

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

export function openSettings(): Promise<void> {
  return invoke<void>("open_settings");
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}
