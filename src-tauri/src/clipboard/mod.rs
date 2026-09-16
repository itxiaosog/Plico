pub mod code;
pub mod files;
pub mod image;
pub mod monitor;
pub mod paste;

use crate::model::ItemType;

/// 轻量类型识别。F7 的类型筛选在 1.0 就要求区分链接与颜色，所以入库时就要打标。
pub fn classify(text: &str) -> ItemType {
    let t = text.trim();

    if is_url(t) {
        return ItemType::Link;
    }
    if is_color(t) {
        return ItemType::Color;
    }
    ItemType::Text
}

fn is_url(t: &str) -> bool {
    // 单行、无空白、以 http(s):// 开头
    !t.is_empty()
        && !t.chars().any(char::is_whitespace)
        && (t.starts_with("http://") || t.starts_with("https://"))
        && t.len() > 10
}

fn is_color(t: &str) -> bool {
    let lower = t.to_ascii_lowercase();

    // #rgb / #rrggbb / #rrggbbaa —— 十六进制里出现任何空白都是非法的，
    // 所以这一支可以直接一刀切。
    if let Some(hex) = lower.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }

    // rgb(...) / rgba(...) / hsl(...) / hsla(...)
    //
    // 这里**不能**沿用「含空白就否决」的规则：`rgb(59, 130, 246)` 逗号后面
    // 带空格是标准写法。改成白名单校验括号内容。
    for prefix in ["rgb(", "rgba(", "hsl(", "hsla("] {
        let Some(inner) = lower.strip_prefix(prefix).and_then(|s| s.strip_suffix(')')) else {
            continue;
        };
        if is_color_args(inner) {
            return true;
        }
    }

    false
}

/// 括号内容校验：必须含数字，且只由数字、分隔符、空白和已知角度单位组成。
///
/// 允许角度单位是因为 CSS 支持 `hsl(120deg 75% 25%)` 这种现代写法；
/// 但只认 deg / grad / rad / turn，避免把 `rgb(foo)` 之类的误判成颜色。
fn is_color_args(inner: &str) -> bool {
    if !inner.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }

    let mut rest = inner;
    while let Some(idx) = rest.find(|c: char| c.is_ascii_alphabetic()) {
        let tail = &rest[idx..];
        let end = tail
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(tail.len());
        if !matches!(&tail[..end], "deg" | "grad" | "rad" | "turn") {
            return false;
        }
        rest = &tail[end..];
    }

    true
}

/// F20 代码识别：给一段文本猜它最像哪种语言。
/// 只用于「这条文本要不要按代码样式展示」的启发式，不进库、不参与去重。
///
/// 返回 None 表示没什么把握 —— 普通文本、链接、颜色走各自的分支。
/// 判据的细节（结构化信号 + 代码骨架加权关键词）见 `code` 模块。
pub fn detect_code_lang(text: &str) -> Option<&'static str> {
    code::detect(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 识别链接() {
        assert_eq!(classify("https://github.com/plico/plico"), ItemType::Link);
        assert_eq!(classify("http://localhost:1420"), ItemType::Link);
        assert_eq!(classify("https://a.com b"), ItemType::Text);
    }

    #[test]
    fn 识别颜色() {
        assert_eq!(classify("#3B82F6"), ItemType::Color);
        assert_eq!(classify("#fff"), ItemType::Color);
        assert_eq!(classify("#3B82F6FF"), ItemType::Color);
        // 函数式写法里逗号后带空格是标准写法，不能因此被否决
        assert_eq!(classify("rgb(59, 130, 246)"), ItemType::Color);
        assert_eq!(classify("hsl(217, 91%, 60%)"), ItemType::Color);
        // 现代 CSS 的角度单位
        assert_eq!(classify("hsl(120deg 75% 25%)"), ItemType::Color);

        assert_eq!(classify("#3B82F"), ItemType::Text);
        // 十六进制里不允许空白
        assert_eq!(classify("#3B82F6 蓝色"), ItemType::Text);
        // 括号里不能是任意字母
        assert_eq!(classify("rgb(foo)"), ItemType::Text);
        assert_eq!(classify("rgb()"), ItemType::Text);
    }

    #[test]
    fn 普通文本() {
        assert_eq!(classify("给张三的邮件草稿"), ItemType::Text);
    }
}
