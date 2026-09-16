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

pub struct AppState {
    pub db: Arc<Db>,
    pub suppress: Arc<Suppress>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub panel_pinned: AtomicBool,
    pub data_dir: PathBuf,
}

impl AppState {
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
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            panel::show_panel(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    hotkey::dispatch(app, shortcut, event.state());
                })
                .build(),
        )
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

            let data_dir = data_dir::resolve(&handle)?;
            std::fs::create_dir_all(&data_dir)?;

            if let Ok(legacy) = data_dir::legacy_dir(&handle) {
                if legacy != data_dir {
                    if let Err(e) = data_dir::migrate(&legacy, &data_dir) {
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
            sync_snippet_hotkeys(&handle, &db, &settings);

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
                    api.prevent_close();
                    settings_window::hide(window);
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("Plico 启动失败");
}

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

fn register_plain_hotkey(app: &AppHandle, settings: &AppSettings) {
    if let Err(e) = hotkey::register(app, &settings.plain_hotkey) {
        eprintln!("[plico] 纯文本热键注册失败：{e}（不影响面板唤起）");
    }
}

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
