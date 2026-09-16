use std::time::Duration;

use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Window,
};

use crate::settings::{self, PanelPosition};
use crate::AppState;

pub const PANEL_LABEL: &str = "panel";
pub const EVENT_PANEL_SHOWN: &str = "plico://panel-shown";

const VERTICAL_ANCHOR: f64 = 0.18;

pub const LIST_MIN_W: f64 = 320.0;
pub const PREVIEW_MIN_W: f64 = 240.0;
const DIVIDER_W: f64 = 1.0;
const MIN_H: f64 = 200.0;

pub fn toggle_panel(app: &AppHandle) {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return;
    };

    if window.is_visible().unwrap_or(false) {
        hide_panel(app);
    } else {
        show_panel(app);
    }
}

pub fn show_panel(app: &AppHandle) {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return;
    };

    restore_size(app, &window);
    place(app, &window);
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit(EVENT_PANEL_SHOWN, ());
}

pub fn hide_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        remember_geometry(app, &window);
        let _ = window.hide();
    }
}

fn restore_size(app: &AppHandle, window: &tauri::WebviewWindow) {
    let settings = app.state::<AppState>().settings();
    let (Some(w), Some(h)) = (settings.panel_w, settings.panel_h) else {
        return;
    };
    let _ = window.set_size(LogicalSize::new(w as f64, h as f64));
}

pub fn apply_min_size(app: &AppHandle, collapsed: bool) {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return;
    };
    let min_w = if collapsed {
        LIST_MIN_W
    } else {
        LIST_MIN_W + PREVIEW_MIN_W + DIVIDER_W
    };
    let _ = window.set_min_size(Some(LogicalSize::new(min_w, MIN_H)));
}

fn place(app: &AppHandle, window: &tauri::WebviewWindow) {
    let settings = app.state::<AppState>().settings();

    if settings.panel_position == PanelPosition::Remember {
        if let (Some(x), Some(y)) = (settings.panel_x, settings.panel_y) {
            if let Ok(Some(_)) = app.monitor_from_point(x as f64, y as f64) {
                let _ = window.set_position(PhysicalPosition::new(x, y));
                return;
            }
        }
    }

    position_near_cursor(app, window);
}

fn position_near_cursor(app: &AppHandle, window: &tauri::WebviewWindow) {
    let Ok(cursor) = app.cursor_position() else {
        return;
    };
    let Ok(Some(monitor)) = app.monitor_from_point(cursor.x, cursor.y) else {
        return;
    };

    let size = window
        .outer_size()
        .unwrap_or_else(|_| PhysicalSize::new(760, 480));

    let mpos = monitor.position();
    let msize = monitor.size();

    let x = mpos.x as f64 + (msize.width as f64 - size.width as f64) / 2.0;
    let y = mpos.y as f64 + msize.height as f64 * VERTICAL_ANCHOR;

    let _ = window.set_position(PhysicalPosition::new(
        x.round() as i32,
        y.round() as i32,
    ));
}

fn remember_geometry(app: &AppHandle, window: &tauri::WebviewWindow) {
    let state = app.state::<AppState>();

    let Ok(pos) = window.outer_position() else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let Ok(size) = window.inner_size() else {
        return;
    };
    let logical_w = (size.width as f64 / scale).round() as i64;
    let logical_h = (size.height as f64 / scale).round() as i64;

    let mut guard = state.settings.write().unwrap_or_else(|e| e.into_inner());

    let mut dirty = false;
    if guard.panel_position == PanelPosition::Remember
        && (guard.panel_x != Some(pos.x) || guard.panel_y != Some(pos.y))
    {
        guard.panel_x = Some(pos.x);
        guard.panel_y = Some(pos.y);
        dirty = true;
    }
    if guard.panel_w != Some(logical_w) || guard.panel_h != Some(logical_h) {
        guard.panel_w = Some(logical_w);
        guard.panel_h = Some(logical_h);
        dirty = true;
    }
    if !dirty {
        return;
    }

    if let Err(e) = settings::save(&state.db, &guard) {
        eprintln!("[plico] 记忆面板位置/尺寸失败：{e}");
    }
}

pub fn handle_focus_lost(window: Window) {
    if window.label() != PANEL_LABEL {
        return;
    }

    let app = window.app_handle().clone();
    let state = app.state::<crate::AppState>();
    if state.panel_pinned.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    let settings = state.settings();
    if !settings.hide_on_blur {
        return;
    }

    let delay = settings.hide_delay_ms.clamp(0, 5_000) as u64;
    if delay == 0 {
        let _ = window.hide();
        return;
    }

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(delay));
        if !window.is_focused().unwrap_or(false) {
            let _ = window.hide();
        }
    });
}
