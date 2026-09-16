use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::app::panel;
use crate::app::settings_window;
use crate::clipboard::monitor::EVENT_ITEMS_CHANGED;
use crate::error::{Error, Result};
use crate::AppState;

pub fn build(app: &AppHandle) -> Result<()> {
    let open = MenuItem::with_id(app, "open", "打开面板", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let clear = MenuItem::with_id(app, "clear", "清空历史", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Plico", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open, &settings_item, &separator, &clear, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| Error::Other("缺少默认窗口图标，无法创建托盘".into()))?;

    TrayIconBuilder::with_id("plico-tray")
        .icon(icon)
        .tooltip("Plico · 剪贴板历史")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => panel::show_panel(app),
            "settings" => {
                if let Err(e) = settings_window::open(app) {
                    eprintln!("[plico] 打开设置失败：{e}");
                }
            }
            "clear" => {
                match app.state::<AppState>().db.clear(true) {
                    Ok(n) => {
                        println!("[plico] 已清空 {n} 条历史（保留置顶）");
                        let _ = app.emit(EVENT_ITEMS_CHANGED, ());
                    }
                    Err(e) => eprintln!("[plico] 清空历史失败：{e}"),
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                panel::toggle_panel(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
