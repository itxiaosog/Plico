use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt as _;

use crate::app::hotkey;
use crate::app::panel;
use crate::app::settings_window::{self, EVENT_SETTINGS_CHANGED};
use crate::clipboard::paste;
use crate::error::{Error, Result};
use crate::model::{Item, ItemType, Rule, Stats};
use crate::settings::{self, AppSettings};
use crate::AppState;

#[tauri::command]
pub fn list_items(
    state: State<'_, AppState>,
    query: Option<String>,
    kind: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<Item>> {
    let kind = kind.as_deref().and_then(ItemType::parse);
    state.db.list(query.as_deref(), kind, limit.unwrap_or(500))
}

#[tauri::command]
pub fn get_item(state: State<'_, AppState>, id: i64) -> Result<Item> {
    state.db.get(id)
}

#[tauri::command]
pub fn delete_item(state: State<'_, AppState>, id: i64) -> Result<()> {
    state.db.delete_with_files(id)
}

#[tauri::command]
pub fn toggle_pin(state: State<'_, AppState>, id: i64) -> Result<Item> {
    state.db.toggle_pin(id)
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>, keep_pinned: Option<bool>) -> Result<usize> {
    state.db.clear(keep_pinned.unwrap_or(true))
}

#[tauri::command]
pub fn copy_text(state: State<'_, AppState>, text: String) -> Result<()> {
    state.suppress.arm_default();
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

#[tauri::command]
pub fn detect_code(text: String) -> Option<String> {
    crate::clipboard::detect_code_lang(&text).map(str::to_string)
}

#[tauri::command]
pub fn get_stats(state: State<'_, AppState>) -> Result<Stats> {
    state.db.stats()
}

#[tauri::command]
pub fn paste_item(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    as_plain_text: Option<bool>,
) -> Result<()> {
    paste::paste_item_as(&app, &state.db, &state.suppress, id, as_plain_text.unwrap_or(false))
}

#[tauri::command]
pub fn paste_text(app: AppHandle, state: State<'_, AppState>, text: String) -> Result<()> {
    paste::paste_text(&app, &state.suppress, &text)
}

#[tauri::command]
pub fn paste_text_with_cursor(app: AppHandle, state: State<'_, AppState>, text: String) -> Result<()> {
    let (text, left_moves) = paste::split_cursor_placeholder(&text);
    paste::paste_text_with_cursor_offset(&app, &state.suppress, &text, left_moves)
}

#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    panel::hide_panel(&app);
}

#[tauri::command]
pub fn toggle_panel_pin(state: State<'_, AppState>) -> bool {
    let pinned = state.panel_pinned.fetch_xor(true, std::sync::atomic::Ordering::Relaxed);
    !pinned
}

#[tauri::command]
pub fn is_panel_pinned(state: State<'_, AppState>) -> bool {
    state.panel_pinned.load(std::sync::atomic::Ordering::Relaxed)
}

#[tauri::command]
pub fn list_snippets(state: State<'_, AppState>) -> Result<Vec<crate::model::Snippet>> { state.db.list_snippets() }

#[tauri::command]
pub fn create_snippet(app: AppHandle, state: State<'_, AppState>, title:String, content:String, tags:Option<String>, shortcut:Option<String>) -> Result<crate::model::Snippet> {
    let shortcut = check_snippet_shortcut(&state, shortcut, None)?;
    let snippet = state.db.create_snippet(&crate::model::NewSnippet{title,content,tags,shortcut})?;
    hotkey::log_snippet_sync(&app);
    Ok(snippet)
}

#[tauri::command]
pub fn update_snippet(app: AppHandle, state: State<'_, AppState>, id:i64, title:String, content:String, tags:Option<String>, shortcut:Option<String>) -> Result<()> {
    let shortcut = check_snippet_shortcut(&state, shortcut, Some(id))?;
    state.db.update_snippet(id, &crate::model::NewSnippet{title,content,tags,shortcut})?;
    hotkey::log_snippet_sync(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_snippet(app: AppHandle, state: State<'_, AppState>, id:i64) -> Result<()> {
    state.db.delete_snippet(id)?;
    hotkey::log_snippet_sync(&app);
    Ok(())
}

fn check_snippet_shortcut(
    state: &AppState,
    shortcut: Option<String>,
    self_id: Option<i64>,
) -> Result<Option<String>> {
    let Some(raw) = shortcut.as_deref().map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let normalized = hotkey::normalize(raw)?;

    let settings = state.settings();
    for (label, other) in [
        ("唤起面板", &settings.hotkey),
        ("粘贴为纯文本", &settings.plain_hotkey),
    ] {
        if hotkey::normalize(other)?.as_deref() == normalized.as_deref() {
            return Err(Error::Other(format!("快捷键 {raw} 已被「{label}」占用")));
        }
    }

    for s in state.db.list_snippets()? {
        if Some(s.id) == self_id {
            continue;
        }
        let Some(other) = s.shortcut.as_deref().map(str::trim).filter(|v| !v.is_empty()) else {
            continue;
        };
        if hotkey::normalize(other)?.as_deref() == normalized.as_deref() {
            return Err(Error::Other(format!("快捷键 {raw} 已被片段「{}」占用", s.title)));
        }
    }

    Ok(normalized)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings()
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: serde_json::Value,
) -> Result<AppSettings> {
    let current = state.settings();
    let next = settings::apply_patch(&current, patch)?;

    if next.hotkey != current.hotkey {
        hotkey::apply(&app, &current.hotkey, &next.hotkey)?;
    }
    if next.autostart != current.autostart {
        apply_autostart(&app, next.autostart)?;
    }
    if next.plain_hotkey != current.plain_hotkey {
        hotkey::apply(&app, &current.plain_hotkey, &next.plain_hotkey)?;
    }
    if next.preview_collapsed != current.preview_collapsed {
        panel::apply_min_size(&app, next.preview_collapsed);
    }

    settings::save(&state.db, &next)?;
    state.set_settings(next.clone());

    if next.hotkey != current.hotkey || next.plain_hotkey != current.plain_hotkey {
        hotkey::log_snippet_sync(&app);
    }

    let _ = app.emit(EVENT_SETTINGS_CHANGED, &next);

    Ok(next)
}

fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<()> {
    let manager = app.autolaunch();
    let outcome = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    outcome.map_err(|e| {
        let verb = if enabled { "开启" } else { "关闭" };
        Error::Other(format!("{verb}开机自启失败：{e}"))
    })
}

#[tauri::command]
pub fn list_rules(state: State<'_, AppState>) -> Result<Vec<Rule>> {
    state.db.list_rules()
}

#[tauri::command]
pub fn add_rule(state: State<'_, AppState>, kind: String, value: String) -> Result<Vec<Rule>> {
    if !Rule::is_valid_kind(&kind) {
        return Err(Error::Other(format!("未知的规则类型：{kind}")));
    }
    if kind == Rule::KIND_CONTENT {
        regex::Regex::new(value.trim())
            .map_err(|e| Error::Other(format!("正则表达式无效：{e}")))?;
    }

    state.db.add_rule(&kind, &value)?;
    state.db.list_rules()
}

#[tauri::command]
pub fn delete_rule(state: State<'_, AppState>, id: i64) -> Result<Vec<Rule>> {
    state.db.delete_rule(id)?;
    state.db.list_rules()
}

#[tauri::command]
pub fn set_rule_enabled(
    state: State<'_, AppState>,
    id: i64,
    enabled: bool,
) -> Result<Vec<Rule>> {
    state.db.set_rule_enabled(id, enabled)?;
    state.db.list_rules()
}

#[tauri::command]
pub fn list_groups(state: State<'_, AppState>) -> Result<Vec<crate::model::Group>> { state.db.list_groups() }
#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<crate::model::Tag>> { state.db.list_tags() }
#[tauri::command]
pub fn assign_item_tag(state: State<'_, AppState>, item_id:i64, name:String) -> Result<Vec<crate::model::Tag>> { state.db.assign_item_tag(item_id,&name) }
#[tauri::command]
pub fn remove_item_tag(state: State<'_, AppState>, item_id:i64, name:String) -> Result<Vec<crate::model::Tag>> { state.db.remove_item_tag(item_id,&name) }
#[tauri::command]
pub fn get_item_tags(state: State<'_, AppState>, item_id:i64) -> Result<Vec<crate::model::Tag>> { state.db.item_tags(item_id) }

#[tauri::command]
pub fn list_tag_stats(state: State<'_, AppState>) -> Result<Vec<crate::model::TagStat>> { state.db.list_tag_stats() }
#[tauri::command]
pub fn rename_tag(state: State<'_, AppState>, id: i64, name: String) -> Result<Vec<crate::model::TagStat>> { state.db.rename_tag(id, &name)?; state.db.list_tag_stats() }
#[tauri::command]
pub fn merge_tags(state: State<'_, AppState>, from: i64, to: i64) -> Result<Vec<crate::model::TagStat>> { state.db.merge_tags(from, to)?; state.db.list_tag_stats() }
#[tauri::command]
pub fn delete_tag(state: State<'_, AppState>, id: i64) -> Result<Vec<crate::model::TagStat>> { state.db.delete_tag(id)?; state.db.list_tag_stats() }

#[tauri::command]
pub fn create_group(state: State<'_, AppState>, name: String, color: Option<String>) -> Result<Vec<crate::model::Group>> { state.db.create_group(&name, color.as_deref())?; state.db.list_groups() }
#[tauri::command]
pub fn rename_group(state: State<'_, AppState>, id: i64, name: String, color: Option<String>) -> Result<Vec<crate::model::Group>> { state.db.rename_group(id, &name, color.as_deref())?; state.db.list_groups() }
#[tauri::command]
pub fn delete_group(state: State<'_, AppState>, id: i64) -> Result<Vec<crate::model::Group>> { state.db.delete_group(id)?; state.db.list_groups() }
#[tauri::command]
pub fn reorder_groups(state: State<'_, AppState>, ids: Vec<i64>) -> Result<Vec<crate::model::Group>> { state.db.reorder_groups(&ids)?; state.db.list_groups() }
#[tauri::command]
pub fn assign_item_group(state: State<'_, AppState>, item_id: i64, group_id: Option<i64>) -> Result<()> { state.db.assign_item_group(item_id, group_id) }

#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<()> {
    settings_window::open(&app)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub db_path: String,
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> Result<AppInfo> {
    let dir = crate::data_dir::resolve(&app)?;
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        db_path: dir.join("plico.db").to_string_lossy().to_string(),
        data_dir: dir.to_string_lossy().to_string(),
    })
}
