//! 设置窗口（F22）。
//!
//! 窗口是**按需创建、关掉只收起来**的：
//!   - 启动时不创建，省掉一份常驻 webview 的内存（面板本身已经占一份了）；
//!   - 第一次打开时创建，之后关闭只是 `hide()`，再打开是热状态，没有重建开销。
//!
//! 之所以不用 `tauri.conf.json` 里声明窗口，就是因为声明式窗口会在启动时
//! 立刻实例化，哪怕用户整个会话都不开设置。

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::Result;

pub const SETTINGS_LABEL: &str = "settings";

/// 设置变更广播。所有窗口都会收到，前端据此同步语言等即时生效项。
pub const EVENT_SETTINGS_CHANGED: &str = "plico://settings-changed";

/// 打开设置窗口：已存在就唤到前台，否则新建。
pub fn open(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        // 可能被最小化过，先还原再聚焦
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    // `WebviewUrl::App` 会自动跟随 devUrl / frontendDist，开发和生产都不用手写地址。
    // 前端靠 `getCurrentWindow().label` 判断该渲染设置界面还是面板。
    let window = WebviewWindowBuilder::new(
        app,
        SETTINGS_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Plico 设置")
    .inner_size(900.0, 640.0)
    .min_inner_size(760.0, 540.0)
    .resizable(true)
    // 先隐藏建窗，show() 时 webview 已导航完成，避免闪白底。
    .visible(false)
    .center()
    .build()?;

    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}

/// 关闭设置窗口时收起而不是销毁 —— 保住 webview，下次打开是瞬时的。
///
/// 调用方需先 `api.prevent_close()`，否则窗口已经被销毁了，`hide()` 无从谈起。
pub fn hide(window: &tauri::Window) {
    let _ = window.hide();
}
