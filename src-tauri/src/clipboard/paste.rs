use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use unicode_segmentation::UnicodeSegmentation;

use crate::clipboard::{files, monitor::{now_ms, Suppress}};
use crate::error::{Error, Result};
use crate::model::ItemType;
use crate::storage::db::Db;

const FOCUS_SETTLE_MS: u64 = 120;

const RESTORE_DELAY_MS: u64 = 600;

enum ClipboardSnapshot {
    Text(String),
    Unavailable,
}

fn snapshot(clipboard: &mut arboard::Clipboard) -> ClipboardSnapshot {
    clipboard
        .get_text()
        .map(ClipboardSnapshot::Text)
        .unwrap_or(ClipboardSnapshot::Unavailable)
}

fn restore(clipboard: &mut arboard::Clipboard, suppress: &Arc<Suppress>, snap: ClipboardSnapshot) {
    let ClipboardSnapshot::Text(text) = snap else {
        return;
    };
    suppress.arm_default();
    let _ = clipboard.set_text(text);
}

fn should_restore(app: &AppHandle) -> bool {
    app.state::<crate::AppState>().settings().restore_clipboard
}

#[allow(dead_code)]
pub fn paste_item(app: &AppHandle, db: &Arc<Db>, suppress: &Arc<Suppress>, id: i64) -> Result<()> {
    paste_item_as(app, db, suppress, id, false)
}

pub fn paste_item_as(
    app: &AppHandle,
    db: &Arc<Db>,
    suppress: &Arc<Suppress>,
    id: i64,
    as_plain: bool,
) -> Result<()> {
    let item = db.get(id)?;

    suppress.arm_default();

    let snapshot_before = {
        let mut probe = arboard::Clipboard::new()?;
        snapshot(&mut probe)
    };

    {
        let mut clipboard = arboard::Clipboard::new()?;
        match item.kind {
            ItemType::Image if !as_plain => {
                let path = item
                    .image_path
                    .as_deref()
                    .ok_or_else(|| Error::Other("图片条目缺少原图路径".into()))?;
                let png = std::fs::read(path)?;
                let img = decode_png(&png)?;
                clipboard.set_image(img)?;
            }
            ItemType::Files if !as_plain => {
                let content = item
                    .content
                    .as_deref()
                    .ok_or_else(|| Error::Other("文件条目缺少路径列表".into()))?;
                let paths = files::existing_paths(content);
                if paths.is_empty() {
                    return Err(Error::Other(
                        "文件已不在原位置（源文件已被删除或移动）".into(),
                    ));
                }
                files::write(&mut clipboard, &paths)?;
            }
            _ => {
                let text = item
                    .plain_text
                    .clone()
                    .or_else(|| item.content.clone())
                    .ok_or_else(|| Error::Other("该条目没有可粘贴的内容".into()))?;
                match (item.kind, item.html_content.as_deref()) {
                    (ItemType::RichText, Some(html)) if !as_plain => {
                        if clipboard
                            .set_html(html.to_string(), Some(text.clone()))
                            .is_err()
                        {
                            clipboard.set_text(text)?;
                        }
                    }
                    _ => clipboard.set_text(text)?,
                }
            }
        }
    }

    if let Some(window) = app.get_webview_window("panel") {
        let _ = window.hide();
    }

    std::thread::sleep(Duration::from_millis(FOCUS_SETTLE_MS));

    simulate_paste()?;

    if should_restore(app) {
        std::thread::sleep(Duration::from_millis(RESTORE_DELAY_MS));
        let mut clipboard = arboard::Clipboard::new()?;
        restore(&mut clipboard, suppress, snapshot_before);
    }

    db.mark_used(id, now_ms())?;
    Ok(())
}

pub const CURSOR_PLACEHOLDER: &str = "{{cursor}}";

pub fn split_cursor_placeholder(content: &str) -> (String, usize) {
    match content.find(CURSOR_PLACEHOLDER) {
        None => (content.to_string(), 0),
        Some(idx) => {
            let before = &content[..idx];
            let after = &content[idx + CURSOR_PLACEHOLDER.len()..];
            let left_moves = after.graphemes(true).count();
            (format!("{before}{after}"), left_moves)
        }
    }
}

pub fn paste_text(app: &AppHandle, suppress: &Arc<Suppress>, text: &str) -> Result<()> {
    paste_text_with_cursor_offset(app, suppress, text, 0)
}

pub fn paste_text_with_cursor_offset(
    app: &AppHandle,
    suppress: &Arc<Suppress>,
    text: &str,
    left_moves: usize,
) -> Result<()> {
    suppress.arm_default();

    let snapshot_before = {
        let mut probe = arboard::Clipboard::new()?;
        snapshot(&mut probe)
    };

    {
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(text)?;
    }

    if let Some(window) = app.get_webview_window("panel") {
        let _ = window.hide();
    }
    std::thread::sleep(Duration::from_millis(FOCUS_SETTLE_MS));
    simulate_paste()?;

    if left_moves > 0 {
        std::thread::sleep(Duration::from_millis(POST_PASTE_SETTLE_MS));
        simulate_left_arrows(left_moves)?;
    }

    if should_restore(app) {
        std::thread::sleep(Duration::from_millis(RESTORE_DELAY_MS));
        let mut clipboard = arboard::Clipboard::new()?;
        restore(&mut clipboard, suppress, snapshot_before);
    }
    Ok(())
}

const POST_PASTE_SETTLE_MS: u64 = 60;

fn decode_png(png: &[u8]) -> Result<arboard::ImageData<'static>> {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .map_err(|e| Error::Other(format!("PNG 解码失败：{e}")))?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    Ok(arboard::ImageData {
        width: w,
        height: h,
        bytes: Cow::Owned(rgba.into_raw()),
    })
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn simulate_paste() -> Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| Error::Input(format!("初始化输入模拟失败：{e}")))?;

    let modifier = if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    };

    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| Error::Input(format!("按下修饰键失败：{e}")))?;
    let result = enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| Error::Input(format!("发送 V 失败：{e}")));
    let _ = enigo.key(modifier, Direction::Release);

    result
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn simulate_paste() -> Result<()> {
    Err(Error::Other("移动端不支持模拟粘贴".into()))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn simulate_left_arrows(count: usize) -> Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| Error::Input(format!("初始化输入模拟失败：{e}")))?;
    for _ in 0..count {
        enigo
            .key(Key::LeftArrow, Direction::Click)
            .map_err(|e| Error::Input(format!("发送 Left 失败：{e}")))?;
    }
    Ok(())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn simulate_left_arrows(_count: usize) -> Result<()> {
    Err(Error::Other("移动端不支持模拟按键".into()))
}

#[cfg(test)]
mod tests {
    use super::{split_cursor_placeholder, CURSOR_PLACEHOLDER};
    use unicode_segmentation::UnicodeSegmentation;

    #[test]
    fn no_placeholder_passthrough() {
        let (text, left) = split_cursor_placeholder("hello world");
        assert_eq!(text, "hello world");
        assert_eq!(left, 0);
    }

    #[test]
    fn placeholder_in_middle() {
        let (text, left) = split_cursor_placeholder("亲爱的{{cursor}}，你好");
        assert_eq!(text, "亲爱的，你好");
        assert_eq!(left, 3);
    }

    #[test]
    fn placeholder_at_end() {
        let (text, left) = split_cursor_placeholder("Hello {{cursor}}");
        assert_eq!(text, "Hello ");
        assert_eq!(left, 0);
    }

    #[test]
    fn placeholder_at_start() {
        let (text, left) = split_cursor_placeholder("{{cursor}}abc");
        assert_eq!(text, "abc");
        assert_eq!(left, 3);
    }

    #[test]
    fn only_first_placeholder_is_consumed() {
        let (text, left) = split_cursor_placeholder("a{{cursor}}b{{cursor}}c");
        let expected_suffix = format!("b{CURSOR_PLACEHOLDER}c");
        assert_eq!(text, format!("a{expected_suffix}"));
        assert_eq!(left, expected_suffix.graphemes(true).count());
    }

    #[test]
    fn multibyte_counts_graphemes_not_bytes() {
        let (text, left) = split_cursor_placeholder("{{cursor}}世界");
        assert_eq!(text, "世界");
        assert_eq!(left, 2);
    }

    #[test]
    fn zwj_emoji_counts_as_one() {
        let family = "👨\u{200D}👩\u{200D}👧";
        assert_eq!(family.chars().count(), 5, "前提：ZWJ 序列是 5 个 scalar");
        let (text, left) = split_cursor_placeholder(&format!("{{{{cursor}}}}{family}"));
        assert_eq!(text, family);
        assert_eq!(left, 1);
    }

    #[test]
    fn combining_mark_counts_as_one() {
        let e_acute = "e\u{0301}";
        assert_eq!(e_acute.chars().count(), 2, "前提：组合字符是 2 个 scalar");
        let (text, left) = split_cursor_placeholder(&format!("{{{{cursor}}}}{e_acute}"));
        assert_eq!(text, e_acute);
        assert_eq!(left, 1);
    }

    #[test]
    fn flag_counts_as_one() {
        let flag = "\u{1F1E8}\u{1F1F3}";
        assert_eq!(flag.chars().count(), 2, "前提：旗帜是 2 个 scalar");
        let (text, left) = split_cursor_placeholder(&format!("{{{{cursor}}}}{flag}"));
        assert_eq!(text, flag);
        assert_eq!(left, 1);
    }

    #[test]
    fn skin_tone_modifier_counts_as_one() {
        let thumb = "\u{1F44D}\u{1F3FD}";
        assert_eq!(thumb.chars().count(), 2, "前提：肤色修饰符是 2 个 scalar");
        let (text, left) = split_cursor_placeholder(&format!("{{{{cursor}}}}{thumb}"));
        assert_eq!(text, thumb);
        assert_eq!(left, 1);
    }

    #[test]
    fn crlf_counts_as_one() {
        let (_, left) = split_cursor_placeholder("{{cursor}}\r\nx");
        assert_eq!(left, 2, "\\r\\n 算 1 次，x 算 1 次");
    }

    #[test]
    fn mixed_content_counts_visual_characters() {
        let tail = "👨\u{200D}👩\u{200D}👧ok e\u{0301}世界\n\u{1F1E8}\u{1F1F3}";
        assert_eq!(tail.chars().count(), 15, "前提：scalar 数是 15");
        let (_, left) = split_cursor_placeholder(&format!("{{{{cursor}}}}{tail}"));
        assert_eq!(left, 9);
    }
}
