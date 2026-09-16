pub mod code;
pub mod files;
pub mod image;
pub mod monitor;
pub mod paste;

use crate::model::ItemType;

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
    !t.is_empty()
        && !t.chars().any(char::is_whitespace)
        && (t.starts_with("http://") || t.starts_with("https://"))
        && t.len() > 10
}

fn is_color(t: &str) -> bool {
    let lower = t.to_ascii_lowercase();

    if let Some(hex) = lower.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }

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
        assert_eq!(classify("rgb(59, 130, 246)"), ItemType::Color);
        assert_eq!(classify("hsl(217, 91%, 60%)"), ItemType::Color);
        assert_eq!(classify("hsl(120deg 75% 25%)"), ItemType::Color);

        assert_eq!(classify("#3B82F"), ItemType::Text);
        assert_eq!(classify("#3B82F6 蓝色"), ItemType::Text);
        assert_eq!(classify("rgb(foo)"), ItemType::Text);
        assert_eq!(classify("rgb()"), ItemType::Text);
    }

    #[test]
    fn 普通文本() {
        assert_eq!(classify("给张三的邮件草稿"), ItemType::Text);
    }
}
