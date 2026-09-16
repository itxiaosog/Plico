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

/// 列表查询。`kind` 传 "all" 或 None 表示不筛选。
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
    // 连带删掉图片文件（F9/F12），孤儿 PNG 会跟着条目一直占磁盘
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

/// F20：把文本写入系统剪贴板，但不模拟粘贴（复制 Markdown 链接等）。
#[tauri::command]
pub fn copy_text(state: State<'_, AppState>, text: String) -> Result<()> {
    state.suppress.arm_default();
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

/// F20：猜一段文本像哪种代码语言。预览区拿它决定要不要切等宽字体展示。
/// 返回 None 表示不像代码。
#[tauri::command]
pub fn detect_code(text: String) -> Option<String> {
    crate::clipboard::detect_code_lang(&text).map(str::to_string)
}

#[tauri::command]
pub fn get_stats(state: State<'_, AppState>) -> Result<Stats> {
    state.db.stats()
}

/// 粘贴选中项：写剪贴板 → 隐藏面板 → 模拟 Ctrl+V。
///
/// `as_plain_text` 为 true 时强制走纯文本（F15）—— 只写 `plain_text`，
/// 对富文本意味着不带 HTML，对图片/文件条目则退化为写文件名/路径文本。
#[tauri::command]
pub fn paste_item(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    as_plain_text: Option<bool>,
) -> Result<()> {
    paste::paste_item_as(&app, &state.db, &state.suppress, id, as_plain_text.unwrap_or(false))
}

/// F19：直接粘一段文本（编辑后的内容）。不走条目 id，不更新 last_used_at。
#[tauri::command]
pub fn paste_text(app: AppHandle, state: State<'_, AppState>, text: String) -> Result<()> {
    paste::paste_text(&app, &state.suppress, &text)
}

/// F18：粘片段内容，支持 `{{cursor}}` 占位符 —— 粘贴后光标停在占位符处。
/// 占位符解析在 Rust 侧做（`split_cursor_placeholder`），前端把原文整段传过来即可。
#[tauri::command]
pub fn paste_text_with_cursor(app: AppHandle, state: State<'_, AppState>, text: String) -> Result<()> {
    let (text, left_moves) = paste::split_cursor_placeholder(&text);
    paste::paste_text_with_cursor_offset(&app, &state.suppress, &text, left_moves)
}

#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    panel::hide_panel(&app);
}

/// 面板「钉住桌面」：钉住时失焦不自动隐藏。返回切换后的新状态。
/// 会话级状态，不持久化到设置 —— 每次启动都是未钉住。
#[tauri::command]
pub fn toggle_panel_pin(state: State<'_, AppState>) -> bool {
    let pinned = state.panel_pinned.fetch_xor(true, std::sync::atomic::Ordering::Relaxed);
    !pinned // 返回新值（翻转后）
}

/// 查询面板当前是否钉住。面板启动时读一次来初始化按钮态。
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
    // 必须重算：不注销的话这个键会一直被 Plico 占着，用户按下去毫无反应，
    // 而且别的程序也注册不了 —— 这是「删了但还占着」的隐蔽残留。
    hotkey::log_snippet_sync(&app);
    Ok(())
}

/// 校验片段快捷键（F18），返回归一化后的写法。
///
/// 冲突在这里就挡掉，而不是等注册时静默失败 —— 全局快捷键撞车是用户唯一
/// 无法从界面上察觉的失败模式（存是存下了，按下去没反应），必须当场说清楚。
///
/// `self_id` 是「正在编辑的片段」，比对时要跳过它自己，否则改内容时
/// 不改快捷键会被自己挡住。
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

/// 更新设置。前端传的是**补丁**（只带改动的字段），合并与钳制都在 Rust 侧做，
/// 返回落库后的权威值 —— 前端拿它覆盖本地状态，就不可能出现「界面显示 9999
/// 但库里其实是 10000」这种不一致。
///
/// 带副作用的两项（热键、开机自启）**先执行成功再落库**：失败就整份放弃，
/// 不留下「设置说开了但实际没开」的状态。
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
    // 折叠预览区之后窗口最小宽度跟着变小（列表独占，320 就够）——
    // 不调这一步，折叠了也拉不窄，用户会觉得「折叠没生效」。
    if next.preview_collapsed != current.preview_collapsed {
        panel::apply_min_size(&app, next.preview_collapsed);
    }

    settings::save(&state.db, &next)?;
    state.set_settings(next.clone());

    // 面板 / 纯文本热键变了，片段热键的「撞键判定」也跟着变：之前被跳过的现在
    // 可能能注册上，之前能注册的现在可能撞上了。必须重算 —— 否则用户把面板热键
    // 从 Ctrl+1 改走之后，那条被跳过的片段键要等到下次重启才生效。
    // 放在 set_settings 之后：sync 读的是 state.settings()。
    if next.hotkey != current.hotkey || next.plain_hotkey != current.plain_hotkey {
        hotkey::log_snippet_sync(&app);
    }

    // 广播给所有窗口：面板要立刻换主题，设置窗口要同步多端改动
    let _ = app.emit(EVENT_SETTINGS_CHANGED, &next);

    Ok(next)
}

/// 开机自启（F11）。失败要把原因带回去，用户在设置页能看到。
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

// ---------------- 隐私规则（F10）----------------

/// 三个变更命令都返回**变更后的完整列表**，前端直接替换本地状态。
/// 这样不用在前端复刻一遍排序/去重逻辑，也就不会和后端跑偏。
#[tauri::command]
pub fn list_rules(state: State<'_, AppState>) -> Result<Vec<Rule>> {
    state.db.list_rules()
}

#[tauri::command]
pub fn add_rule(state: State<'_, AppState>, kind: String, value: String) -> Result<Vec<Rule>> {
    if !Rule::is_valid_kind(&kind) {
        return Err(Error::Other(format!("未知的规则类型：{kind}")));
    }
    // 内容规则先试编译一次：坏正则存进去只会在每次复制时静默失效，
    // 用户完全看不出问题。宁可现在报错。
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

// ---- F17 标签管理（设置窗口的标签页）----
// 这四个都返回变更后的完整统计列表：标签页是「一屏看全」的形态，
// 每次改完让后端回权威数据，比前端自己打补丁可靠。

#[tauri::command]
pub fn list_tag_stats(state: State<'_, AppState>) -> Result<Vec<crate::model::TagStat>> { state.db.list_tag_stats() }
#[tauri::command]
pub fn rename_tag(state: State<'_, AppState>, id: i64, name: String) -> Result<Vec<crate::model::TagStat>> { state.db.rename_tag(id, &name)?; state.db.list_tag_stats() }
#[tauri::command]
pub fn merge_tags(state: State<'_, AppState>, from: i64, to: i64) -> Result<Vec<crate::model::TagStat>> { state.db.merge_tags(from, to)?; state.db.list_tag_stats() }
#[tauri::command]
pub fn delete_tag(state: State<'_, AppState>, id: i64) -> Result<Vec<crate::model::TagStat>> { state.db.delete_tag(id)?; state.db.list_tag_stats() }

// ---- F16 分组管理（面板的筛选菜单）----
// 三个写操作都返回**变更后的完整分组列表**：和 F17 标签页同一立场，
// 让后端回权威数据比前端自己打补丁可靠（新建要知道新 id，删除要清选中项）。

#[tauri::command]
pub fn create_group(state: State<'_, AppState>, name: String, color: Option<String>) -> Result<Vec<crate::model::Group>> { state.db.create_group(&name, color.as_deref())?; state.db.list_groups() }
#[tauri::command]
pub fn rename_group(state: State<'_, AppState>, id: i64, name: String, color: Option<String>) -> Result<Vec<crate::model::Group>> { state.db.rename_group(id, &name, color.as_deref())?; state.db.list_groups() }
#[tauri::command]
pub fn delete_group(state: State<'_, AppState>, id: i64) -> Result<Vec<crate::model::Group>> { state.db.delete_group(id)?; state.db.list_groups() }
/// 按传入的完整 id 顺序重排分组（F16）。返回变更后的完整列表。
#[tauri::command]
pub fn reorder_groups(state: State<'_, AppState>, ids: Vec<i64>) -> Result<Vec<crate::model::Group>> { state.db.reorder_groups(&ids)?; state.db.list_groups() }
#[tauri::command]
pub fn assign_item_group(state: State<'_, AppState>, item_id: i64, group_id: Option<i64>) -> Result<()> { state.db.assign_item_group(item_id, group_id) }


/// 打开设置窗口（F22）。
///
/// **必须是 async 命令**：`WebviewWindowBuilder::build()` 在 Windows 上
/// 用于同步命令时会死锁（WebView2 已知问题），导致设置窗口卡在 about:blank。
/// 改成 async 后 `build()` 在 async runtime 上执行，不阻塞主线程事件循环。
#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<()> {
    settings_window::open(&app)
}

/// 关于页（F22）需要的信息。
///
/// 数据目录是**固定值**（安装目录下的 `data/`），这里只用来展示路径 ——
/// 1.x 那条「用户自选数据目录」的路已经取消，没有对应的写入口了。
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
