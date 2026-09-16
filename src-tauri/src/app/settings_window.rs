use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::Result;

pub const SETTINGS_LABEL: &str = "settings";

pub const EVENT_SETTINGS_CHANGED: &str = "plico://settings-changed";

pub fn open(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        SETTINGS_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Plico 设置")
    .inner_size(900.0, 640.0)
    .min_inner_size(760.0, 540.0)
    .resizable(true)
    .visible(false)
    .center()
    .build()?;

    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}

pub fn hide(window: &tauri::Window) {
    let _ = window.hide();
}
