use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use unicode_segmentation::UnicodeSegmentation;

use crate::clipboard::{files, monitor::{now_ms, Suppress}};
use crate::error::{Error, Result};
use crate::model::ItemType;
use crate::storage::db::Db;

/// 隐藏面板后等焦点切回原窗口的时间。太短会把 Ctrl+V 发到面板自己身上。
const FOCUS_SETTLE_MS: u64 = 120;

/// F21「粘贴后恢复剪贴板」：模拟 Ctrl+V 之后等目标应用真的读走剪贴板，
/// 再把剪贴板还原成粘贴前的内容。太短可能赶不上应用的读取节奏。
const RESTORE_DELAY_MS: u64 = 600;

/// 粘贴前剪贴板里有什么，决定要不要恢复以及怎么恢复。
///
/// 只处理文本：图片/文件恢复的成本与风险都高（CF_HDROP 的句柄生命周期、
/// 大位图二次编码），而 F21 的动机是「敏感文本别在剪贴板里挂太久」，
/// 覆盖文本场景就够了。
enum ClipboardSnapshot {
    Text(String),
    /// 拿不到文本（剪贴板是图片/文件/空的）。不恢复 —— 恢复意味着要往
    /// 剪贴板里写东西，写错了比不恢复更糟。
    Unavailable,
}

fn snapshot(clipboard: &mut arboard::Clipboard) -> ClipboardSnapshot {
    clipboard
        .get_text()
        .map(ClipboardSnapshot::Text)
        .unwrap_or(ClipboardSnapshot::Unavailable)
}

/// 开启抑制窗口后按快照恢复剪贴板。抑制必须包住恢复写入，
/// 否则恢复会被监听器当成一次新复制入库。
fn restore(clipboard: &mut arboard::Clipboard, suppress: &Arc<Suppress>, snap: ClipboardSnapshot) {
    let ClipboardSnapshot::Text(text) = snap else {
        return;
    };
    suppress.arm_default();
    let _ = clipboard.set_text(text);
}

/// 判断是否要在延迟后恢复剪贴板（设置项 F21，默认关）。
fn should_restore(app: &AppHandle) -> bool {
    app.state::<crate::AppState>().settings().restore_clipboard
}

/// 执行一次粘贴：写剪贴板 → 隐藏面板 → 模拟 Ctrl+V → 记录使用时间。
///
/// 关键顺序：抑制窗口必须在写剪贴板 **之前** 开启，否则监听线程可能在
/// arm 之前就看到了这次变化。
///
/// `paste_item_as` 的便捷包装。真正的入口是带 `as_plain` 的那个。
#[allow(dead_code)]
pub fn paste_item(app: &AppHandle, db: &Arc<Db>, suppress: &Arc<Suppress>, id: i64) -> Result<()> {
    paste_item_as(app, db, suppress, id, false)
}

/// `as_plain` 为 true 时强制走纯文本分支（F15）。
/// 对富文本条目意味着只写 `plain_text`，不带 `html_content`；
/// 对图片/文件条目等于退化成「写文本摘要」—— 调用方（Ctrl+Enter /
/// 全局热键 / 上下文菜单）本身就只该对文本类条目用它。
///
/// 富文本（F19 规格「粘贴时保留格式」）：有 `html_content` 时写
/// CF_HTML + 纯文本两个格式。认识 HTML 的目标应用（Word/邮件/网页编辑器）
/// 粘出格式，不认识的（记事本/终端）落到纯文本替代内容，两边都不落空。
pub fn paste_item_as(
    app: &AppHandle,
    db: &Arc<Db>,
    suppress: &Arc<Suppress>,
    id: i64,
    as_plain: bool,
) -> Result<()> {
    let item = db.get(id)?;

    // 1) 先开抑制窗口 —— 三种类型写剪贴板前都得挡住监听
    suppress.arm_default();

    // F21：写入之前先记下剪贴板现状，粘贴完成后再还原
    let snapshot_before = {
        let mut probe = arboard::Clipboard::new()?;
        snapshot(&mut probe)
    };

    // 2) 按条目类型写入系统剪贴板
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
                    // F13：源文件全被删/移走时提示，而不是悄悄放一个空剪贴板
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
                    // set_html 内部先写 CF_HTML 再写 alt 纯文本；arboard 只在两者
                    // 都失败时才报错。走到 Err 说明连纯文本都没写上，再单独试一次。
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

    // 3) 隐藏面板，把焦点还给用户原来所在的窗口
    if let Some(window) = app.get_webview_window("panel") {
        let _ = window.hide();
    }

    // 4) 等焦点稳定
    std::thread::sleep(Duration::from_millis(FOCUS_SETTLE_MS));

    // 5) 模拟按键。失败时不回滚 —— 内容已经在剪贴板里了，用户手动 Ctrl+V 即可（R2 的对策）
    simulate_paste()?;

    // 6) F21：开启时把剪贴板恢复为粘贴前的内容。
    //    恢复本身也是一次 Plico 写入，必须再 arm 一次抑制窗口。
    if should_restore(app) {
        std::thread::sleep(Duration::from_millis(RESTORE_DELAY_MS));
        let mut clipboard = arboard::Clipboard::new()?;
        restore(&mut clipboard, suppress, snapshot_before);
    }

    db.mark_used(id, now_ms())?;
    Ok(())
}

/// F18 `{{cursor}}` 占位符。片段内容里出现一次时，粘贴后光标要停在它原来的位置。
pub const CURSOR_PLACEHOLDER: &str = "{{cursor}}";

/// 把含占位符的片段内容拆成「要粘的纯文本」+「粘贴后需要左移的**字素簇**个数」。
///
/// - 没有占位符：原样返回，左移 0。
/// - 有占位符：取**第一个**出现的位置，删掉它，左移数 = 占位符之后那段文本的
///   字素簇个数。
///
/// **为什么按字素簇数而不是 `char` 数**：Left 方向键在编辑器里按「一个视觉
/// 字符」走，而 `char` 数的是 Unicode scalar。下列场景两者会差出来，按 `char`
/// 数就会多按几次 Left，光标落到占位符**左边**：
///
/// | 文本 | scalar 数 | 视觉字符数 |
/// | --- | --- | --- |
/// | `👨‍👩‍👧`（ZWJ 序列） | 5 | 1 |
/// | `e` + U+0301（组合字符） | 2 | 1 |
/// | `🇨🇳`（两个 regional indicator） | 2 | 1 |
/// | `👍🏽`（+ 肤色修饰符） | 2 | 1 |
///
/// 反过来 `\r\n` 是 2 个 scalar 但 1 个字素簇，而 Left 在行首跳上一行末尾也
/// 只算一次 —— 字素簇数同样是对的。
///
/// 多余的占位符按普通文本留在内容里（和旧行为 `replace(all)` 不同：旧行为会
/// 把它们也删掉，但「光标只能停一处」，留原文比悄悄删字更不容易误解）。
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

/// 把任意一段文本直接写剪贴板并模拟粘贴（F19 编辑后的内容）。
/// 不入库为已有条目 —— 它是「这次要粘的内容」，不是历史。
pub fn paste_text(app: &AppHandle, suppress: &Arc<Suppress>, text: &str) -> Result<()> {
    paste_text_with_cursor_offset(app, suppress, text, 0)
}

/// F18：粘一段文本，粘贴完成后把光标向左移 `left_moves` 个**字素簇**。
///
/// 实现方式：整段一次 Ctrl+V 粘进去，再模拟 N 次 Left 方向键。
/// 比分两次粘贴（前缀 → 模拟 → 后缀）稳：两次 Ctrl+V 之间目标应用可能
/// 还没读完剪贴板，第二次写入会覆盖掉还没粘走的前缀；而方向键是目标应用
/// 自己处理的，不碰剪贴板时序。
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

    // 等目标应用把粘贴内容处理完再移光标，否则 Left 可能打在粘贴之前。
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

/// Ctrl+V 之后等目标应用把文本落进文档的时间。太短方向键会打在粘贴完成之前。
const POST_PASTE_SETTLE_MS: u64 = 60;

/// PNG 字节 → arboard 的 RGBA `ImageData`。
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
    // 无论成功与否都要松开修饰键，否则用户的 Ctrl 会卡住
    let _ = enigo.key(modifier, Direction::Release);

    result
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn simulate_paste() -> Result<()> {
    Err(Error::Other("移动端不支持模拟粘贴".into()))
}

/// F18：模拟 N 次 Left 方向键，把光标挪回 `{{cursor}}` 占位符的位置。
///
/// `count` 的单位是**字素簇**（见 `split_cursor_placeholder` 的说明）——
/// 一次按键对应一个视觉字符，而不是一个 Unicode scalar。
///
/// 逐次 Click 而不是按住 —— 按住会触发系统自动重复，次数不可控。
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
        // 后缀「，你好」3 个字符 → 左移 3 次
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
        // 第二个占位符按普通文本保留，且计入左移长度
        let expected_suffix = format!("b{CURSOR_PLACEHOLDER}c");
        assert_eq!(text, format!("a{expected_suffix}"));
        assert_eq!(left, expected_suffix.graphemes(true).count());
    }

    #[test]
    fn multibyte_counts_graphemes_not_bytes() {
        // "世界" 是 2 个视觉字符 / 6 个字节；方向键按视觉字符走
        let (text, left) = split_cursor_placeholder("{{cursor}}世界");
        assert_eq!(text, "世界");
        assert_eq!(left, 2);
    }

    /// 方向键按「一个视觉字符」走，所以一个 emoji ZWJ 序列只算一次左移。
    /// 这一条是本次修正的核心 —— 旧实现用 `chars().count()` 会得到 5。
    #[test]
    fn zwj_emoji_counts_as_one() {
        let family = "👨\u{200D}👩\u{200D}👧";
        assert_eq!(family.chars().count(), 5, "前提：ZWJ 序列是 5 个 scalar");
        let (text, left) = split_cursor_placeholder(&format!("{{{{cursor}}}}{family}"));
        assert_eq!(text, family);
        assert_eq!(left, 1);
    }

    /// 组合字符（基字符 + 变音符号）也是 2 个 scalar / 1 个视觉字符。
    #[test]
    fn combining_mark_counts_as_one() {
        let e_acute = "e\u{0301}";
        assert_eq!(e_acute.chars().count(), 2, "前提：组合字符是 2 个 scalar");
        let (text, left) = split_cursor_placeholder(&format!("{{{{cursor}}}}{e_acute}"));
        assert_eq!(text, e_acute);
        assert_eq!(left, 1);
    }

    /// 旗帜 = 两个 regional indicator（4 个 UTF-16 code unit），仍是 1 个视觉字符。
    #[test]
    fn flag_counts_as_one() {
        let flag = "\u{1F1E8}\u{1F1F3}";
        assert_eq!(flag.chars().count(), 2, "前提：旗帜是 2 个 scalar");
        let (text, left) = split_cursor_placeholder(&format!("{{{{cursor}}}}{flag}"));
        assert_eq!(text, flag);
        assert_eq!(left, 1);
    }

    /// 带肤色修饰符的 emoji 同理。
    #[test]
    fn skin_tone_modifier_counts_as_one() {
        let thumb = "\u{1F44D}\u{1F3FD}";
        assert_eq!(thumb.chars().count(), 2, "前提：肤色修饰符是 2 个 scalar");
        let (text, left) = split_cursor_placeholder(&format!("{{{{cursor}}}}{thumb}"));
        assert_eq!(text, thumb);
        assert_eq!(left, 1);
    }

    /// `\r\n` 是 2 个 scalar 但 1 个字素簇，而 Left 在行首跳上一行末尾也只算一次。
    #[test]
    fn crlf_counts_as_one() {
        let (_, left) = split_cursor_placeholder("{{cursor}}\r\nx");
        assert_eq!(left, 2, "\\r\\n 算 1 次，x 算 1 次");
    }

    /// 混合场景：emoji + 组合字符 + 汉字 + 换行，逐段核对总数。
    #[test]
    fn mixed_content_counts_visual_characters() {
        // 👨‍👩‍👧 (1) + "ok" (2) + 空格 (1) + é (1) + "世界" (2) + "\n" (1) + 🇨🇳 (1) = 9
        // 对照：按 Unicode scalar 数是 5+2+1+2+2+1+2 = 15
        let tail = "👨\u{200D}👩\u{200D}👧ok e\u{0301}世界\n\u{1F1E8}\u{1F1F3}";
        assert_eq!(tail.chars().count(), 15, "前提：scalar 数是 15");
        let (_, left) = split_cursor_placeholder(&format!("{{{{cursor}}}}{tail}"));
        assert_eq!(left, 9);
    }
}
