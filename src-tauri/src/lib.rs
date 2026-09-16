mod app;
mod clipboard;
mod commands;
mod data_dir;
mod error;
mod model;
mod platform;
mod privacy;
mod settings;
mod storage;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt as _;

use crate::app::{hotkey, panel, settings_window, tray};
use crate::clipboard::monitor::{self, Suppress};
use crate::settings::AppSettings;
use crate::storage::db::Db;

/// 全局状态。db / suppress / settings 都用 Arc 共享给监听线程。
///
/// `settings` 用 `RwLock` 而不是把值复制进监听线程：设置改完要**立刻**影响
/// 监听行为（比如刚加了一条隐私规则），复制一份就再也同步不上了。
pub struct AppState {
    pub db: Arc<Db>,
    pub suppress: Arc<Suppress>,
    pub settings: Arc<RwLock<AppSettings>>,
    /// 面板「钉住桌面」状态（会话级，不持久化）。钉住时失焦不自动隐藏。
    pub panel_pinned: AtomicBool,
    /// 数据目录（安装目录下的 `data/`，或回退的 `%APPDATA%`）。
    ///
    /// 启动时解析一次存下来：进程生命周期内它不会变，而解析过程带一次写探针，
    /// 不该每次存图片都做一遍。
    pub data_dir: PathBuf,
}

impl AppState {
    /// 读一份设置快照。锁中毒时取内层值继续用 —— 中毒只意味着某个写者 panic 过，
    /// 值本身仍是完整的（写者总是整份替换）。
    pub fn settings(&self) -> AppSettings {
        self.settings
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_settings(&self, next: AppSettings) {
        *self.settings.write().unwrap_or_else(|e| e.into_inner()) = next;
    }
}

pub fn run() {
    tauri::Builder::default()
        // 单实例必须在最前面注册：剪贴板工具跑两份会互相抢剪贴板（F11）
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            panel::show_panel(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    // 只响应按下，否则一次按键会触发 Press + Release 两次切换
                    hotkey::dispatch(app, shortcut, event.state());
                })
                .build(),
        )
        // F11 开机自启
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::list_items,
            commands::get_item,
            commands::delete_item,
            commands::toggle_pin,
            commands::clear_history,
            commands::get_stats,
            commands::paste_item,
            commands::paste_text,
            commands::paste_text_with_cursor,
            commands::copy_text,
            commands::detect_code,
            commands::hide_panel,
            commands::toggle_panel_pin,
            commands::is_panel_pinned,
            commands::get_settings,
            commands::update_settings,
            commands::list_snippets,
            commands::create_snippet,
            commands::update_snippet,
            commands::delete_snippet,
            commands::list_groups,
            commands::list_tags,
            commands::assign_item_tag,
            commands::remove_item_tag,
            commands::get_item_tags,
            commands::list_tag_stats,
            commands::rename_tag,
            commands::merge_tags,
            commands::delete_tag,
            commands::create_group,
            commands::rename_group,
            commands::delete_group,
            commands::reorder_groups,
            commands::assign_item_group,
            commands::list_rules,
            commands::add_rule,
            commands::delete_rule,
            commands::set_rule_enabled,
            commands::open_settings,
            commands::get_app_info,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // 数据目录：安装目录下的 `data/`（装到 Program Files 之类不可写的
            // 位置时自动回退到 `%APPDATA%/com.plico.app/`，见 data_dir::resolve）。
            let data_dir = data_dir::resolve(&handle)?;
            std::fs::create_dir_all(&data_dir)?;

            // 1.x 的数据在 `%APPDATA%/com.plico.app/`，升级到新版本要搬过来。
            // migrate 是幂等的：目标已有库就不搬，旧目录原样保留当备份。
            if let Ok(legacy) = data_dir::legacy_dir(&handle) {
                if legacy != data_dir {
                    if let Err(e) = data_dir::migrate(&legacy, &data_dir) {
                        // 搬失败不改路径 —— 新目录能用就继续用，旧数据留在原地，
                        // 用户下次启动会再试一遍
                        eprintln!("[plico] 旧数据搬迁失败：{e}");
                    }
                }
            }

            let db = Arc::new(Db::open(&data_dir)?);
            let mut settings = settings::load(&db)?;
            let suppress = Arc::new(Suppress::new());

            reconcile_autostart(&handle, &db, &mut settings);
            register_hotkey(&handle, &db, &mut settings);
            register_plain_hotkey(&handle, &settings);
            // F18：片段热键必须排在面板 / 纯文本之后 —— sync 要靠这两个键判断撞车，
            // 顺序反了会把片段键注册到主热键头上。
            sync_snippet_hotkeys(&handle, &db, &settings);

            // 保留策略清理：启动时先扫一遍（F14）。之后由监听线程每 6 小时跑一次。
            match db.purge_expired(settings.retention_days, monitor::now_ms()) {
                Ok(0) => {}
                Ok(n) => println!("[plico] 启动清理：删除 {n} 条过期记录"),
                Err(e) => eprintln!("[plico] 启动清理失败：{e}"),
            }

            let settings = Arc::new(RwLock::new(settings));
            app.manage(AppState {
                db: db.clone(),
                suppress: suppress.clone(),
                settings: settings.clone(),
                panel_pinned: AtomicBool::new(false),
                data_dir: data_dir.clone(),
            });

            tray::build(&handle)?;
            // 预览区折叠状态会影响窗口最小宽度，启动时先对齐一次
            let collapsed = handle.state::<AppState>().settings().preview_collapsed;
            panel::apply_min_size(&handle, collapsed);
            monitor::spawn(handle.clone(), db, suppress, settings);

            println!("[plico] 已启动，数据目录：{}", data_dir.display());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(false) => panel::handle_focus_lost(window.clone()),
            WindowEvent::CloseRequested { api, .. } => {
                if window.label() == settings_window::SETTINGS_LABEL {
                    // 设置窗口关掉只是收起来，下次打开不用重建 webview
                    api.prevent_close();
                    settings_window::hide(window);
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("Plico 启动失败");
}

/// 注册全局热键，注册不上就回退到默认值。
///
/// **注册失败不阻断启动** —— 最坏情况只是没有全局唤起，托盘还在，
/// 为了一个热键让整个剪贴板工具起不来是不划算的。
///
/// 回退只处理**唤起面板**这一条；纯文本热键注册失败只是功能缺失，
/// 不值得为它回退到默认值（那反而会让两个热键撞在一起）。
fn register_hotkey(app: &AppHandle, db: &Db, settings: &mut AppSettings) {
    if hotkey::register(app, &settings.hotkey).is_ok() {
        return;
    }

    let fallback = hotkey::default_spec().to_string();
    eprintln!(
        "[plico] 热键「{}」注册失败（可能已被其它程序占用）",
        settings.hotkey
    );

    if settings.hotkey == fallback {
        eprintln!("[plico] 默认热键 {fallback} 也注册不上，本次只能靠托盘唤起");
        return;
    }

    match hotkey::register(app, &fallback) {
        Ok(()) => {
            eprintln!("[plico] 已回退到默认热键 {fallback}");
            settings.hotkey = fallback;
            if let Err(e) = settings::save(db, settings) {
                eprintln!("[plico] 回退后的热键落库失败：{e}");
            }
        }
        Err(e) => eprintln!("[plico] 默认热键 {fallback} 也注册失败：{e}。只能靠托盘唤起了。"),
    }
}

/// 注册「粘贴为纯文本」全局热键（F15）。失败只记日志，不回退默认值。
fn register_plain_hotkey(app: &AppHandle, settings: &AppSettings) {
    if let Err(e) = hotkey::register(app, &settings.plain_hotkey) {
        eprintln!("[plico] 纯文本热键注册失败：{e}（不影响面板唤起）");
    }
}

/// 注册所有片段快捷键（F18）。
///
/// 启动时直接走 `hotkey::sync_snippets` 而不是 `sync_snippets_from_db`：
/// 此刻 `AppState` 还没 `manage` 进去，拿不到 `state.db`。
fn sync_snippet_hotkeys(app: &AppHandle, db: &Db, settings: &AppSettings) {
    let snippets = match db.list_snippets() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[plico] 读取片段失败，跳过片段热键：{e}");
            return;
        }
    };
    let reserved = vec![settings.hotkey.clone(), settings.plain_hotkey.clone()];
    for w in hotkey::sync_snippets(app, &snippets, &reserved) {
        eprintln!("[plico] {w}");
    }
}

/// 开机自启以**系统实际状态**为准回写一次。
///
/// 用户可能在任务管理器里手动关掉过，那样设置页会显示「已开启」而实际没开。
///
/// 只在 release 构建里做这件事：debug 构建（`tauri dev`）跑的是另一个 exe 路径，
/// 插件的 `is_enabled` 会如实报告「没为这个 exe 注册过」，把用户为正式版设的开
/// 关改掉。开发期不该有这种副作用。
fn reconcile_autostart(app: &AppHandle, db: &Db, settings: &mut AppSettings) {
    if cfg!(debug_assertions) {
        return;
    }

    match app.autolaunch().is_enabled() {
        Ok(actual) if actual != settings.autostart => {
            println!(
                "[plico] 开机自启状态与系统不一致（系统：{actual}），以系统为准",
            );
            settings.autostart = actual;
            if let Err(e) = settings::save(db, settings) {
                eprintln!("[plico] 自启状态落库失败：{e}");
            }
        }
        Ok(_) => {}
        Err(e) => eprintln!("[plico] 读取开机自启状态失败：{e}"),
    }
}
