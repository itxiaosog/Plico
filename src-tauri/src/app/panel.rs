use std::time::Duration;

use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Window,
};

use crate::settings::{self, PanelPosition};
use crate::AppState;

pub const PANEL_LABEL: &str = "panel";
pub const EVENT_PANEL_SHOWN: &str = "plico://panel-shown";

/// 面板纵向位置：屏幕高度的 18% 处。偏上但不贴顶，视线落点最省力。
const VERTICAL_ANCHOR: f64 = 0.18;

/// 列表区最小宽度（F5）。预览区折叠时列表独占窗口，这就是窗口能有多窄。
pub const LIST_MIN_W: f64 = 320.0;
/// 预览区最小宽度（F5）。展开时窗口还要塞下它和一条分隔线。
pub const PREVIEW_MIN_W: f64 = 240.0;
/// 分隔线的宽度。0.5px 在逻辑坐标里算 1。
const DIVIDER_W: f64 = 1.0;
/// 窗口最小高度。低于这个高度连搜索栏 + 一行 + 状态栏都放不下。
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

    // 尺寸要先于位置恢复：`position_near_cursor` 要按窗口宽度算水平居中，
    // 顺序反了面板会按旧宽度居中，然后跳到偏一边。
    restore_size(app, &window);
    place(app, &window);
    let _ = window.show();
    let _ = window.set_focus();
    // 通知前端重置搜索与选中项
    let _ = window.emit(EVENT_PANEL_SHOWN, ());
}

pub fn hide_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        remember_geometry(app, &window);
        let _ = window.hide();
    }
}

/// 把记忆的尺寸套回窗口。
///
/// 存的是**逻辑像素**，所以要用 `set_size(LogicalSize)` 让 Tauri 按当前
/// 缩放比例换算 —— 换成物理像素的话，把面板搬到另一块 150% 的屏幕上会突然变大。
fn restore_size(app: &AppHandle, window: &tauri::WebviewWindow) {
    let settings = app.state::<AppState>().settings();
    let (Some(w), Some(h)) = (settings.panel_w, settings.panel_h) else {
        return;
    };
    let _ = window.set_size(LogicalSize::new(w as f64, h as f64));
}

/// 按预览区是否折叠设置窗口最小宽度（F5）。
///
/// 折叠时列表独占窗口，320 就够；展开时还要放下预览区的 240 和一条分隔线。
/// 声明式配置里的 `minWidth: 560` 只是启动瞬间的值，之后由这里接管。
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

/// 按设置里的位置模式摆放面板（F5）。
fn place(app: &AppHandle, window: &tauri::WebviewWindow) {
    let settings = app.state::<AppState>().settings();

    if settings.panel_position == PanelPosition::Remember {
        if let (Some(x), Some(y)) = (settings.panel_x, settings.panel_y) {
            // 记住的坐标可能落在已经拔掉的显示器上。校验一下，
            // 落不到任何屏幕就退回跟随光标，免得面板出现在看不见的地方。
            if let Ok(Some(_)) = app.monitor_from_point(x as f64, y as f64) {
                let _ = window.set_position(PhysicalPosition::new(x, y));
                return;
            }
        }
    }

    position_near_cursor(app, window);
}

/// 把面板放到「鼠标所在那块屏幕」的水平居中、垂直偏上位置。
/// 多屏场景下必须按光标所在屏幕算，否则副屏唤起会飞到主屏去。
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

/// 隐藏时把坐标和尺寸落库。
///
/// 坐标只在「记忆位置」模式下才存；**尺寸任何模式下都存** —— 位置模式说的是
/// 「面板出现在哪里」，和「面板多大」是两件事，跟随光标时也该记住用户拉过的尺寸。
///
/// 没变化就不写库，避免每次隐藏都来一次事务。
fn remember_geometry(app: &AppHandle, window: &tauri::WebviewWindow) {
    let state = app.state::<AppState>();

    let Ok(pos) = window.outer_position() else {
        return;
    };
    // 逻辑尺寸 = 物理尺寸 / 缩放比例
    let scale = window.scale_factor().unwrap_or(1.0);
    let Ok(size) = window.inner_size() else {
        return;
    };
    let logical_w = (size.width as f64 / scale).round() as i64;
    let logical_h = (size.height as f64 / scale).round() as i64;

    // 持写锁做「读-改-写」：设置窗口可能正在改别的字段，分两步会把对方的改动覆盖掉。
    // 代价是这期间监听线程读设置会被挡住，但隐藏面板是人类节奏，几毫秒的事。
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

/// 失焦延迟隐藏。只对面板生效——设置窗口失焦不该消失。
pub fn handle_focus_lost(window: Window) {
    if window.label() != PANEL_LABEL {
        return;
    }

    let app = window.app_handle().clone();
    let state = app.state::<crate::AppState>();
    // 「钉住桌面」时跳过失焦隐藏 —— 用户要面板常驻
    if state.panel_pinned.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    let settings = state.settings();
    if !settings.hide_on_blur {
        return;
    }

    // 0 表示立刻隐藏，就不必起线程等一轮了
    let delay = settings.hide_delay_ms.clamp(0, 5_000) as u64;
    if delay == 0 {
        let _ = window.hide();
        return;
    }

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(delay));
        // 延迟期间用户可能又把焦点点回来了，所以要复查一次
        if !window.is_focused().unwrap_or(false) {
            let _ = window.hide();
        }
    });
}
