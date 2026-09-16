use regex::Regex;

use crate::model::Rule;

pub fn is_blocked(rules: &[Rule], source_app: Option<&str>, text: &str) -> bool {
    rules.iter().any(|rule| match rule.kind.as_str() {
        Rule::KIND_APP => matches_app(&rule.value, source_app),
        Rule::KIND_CONTENT => matches_content(&rule.value, text),
        _ => false,
    })
}

fn matches_app(rule_value: &str, source_app: Option<&str>) -> bool {
    let Some(source) = source_app else {
        return false;
    };
    normalize_app(rule_value) == normalize_app(source)
}

fn normalize_app(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    lower
        .strip_suffix(".exe")
        .unwrap_or(lower.as_str())
        .to_string()
}

fn matches_content(pattern: &str, text: &str) -> bool {
    match Regex::new(pattern) {
        Ok(re) => re.is_match(text),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(kind: &str, value: &str) -> Rule {
        Rule {
            id: 1,
            kind: kind.into(),
            value: value.into(),
            enabled: true,
        }
    }

    #[test]
    fn 按进程名排除_忽略大小写与exe后缀() {
        let rules = vec![rule("app", "1Password.exe")];
        assert!(is_blocked(&rules, Some("1password.exe"), "任意内容"));
        assert!(is_blocked(&rules, Some("1Password.exe"), "任意内容"));
        assert!(is_blocked(&rules, Some("1Password"), "填了不带后缀的写法"));
        assert!(!is_blocked(&rules, Some("chrome.exe"), "任意内容"));
    }

    #[test]
    fn 拿不到来源进程时不误杀() {
        let rules = vec![rule("app", "1Password.exe")];
        assert!(!is_blocked(&rules, None, "任意内容"));
    }

    #[test]
    fn 按正则排除内容() {
        let rules = vec![rule("content", r"\d{16}")];
        assert!(is_blocked(&rules, Some("chrome.exe"), "卡号 6222021234567890"));
        assert!(!is_blocked(&rules, Some("chrome.exe"), "只有 15 位 622202123456789"));
    }

    #[test]
    fn 坏正则不命中也不panic() {
        let rules = vec![rule("content", "([unclosed")];
        assert!(!is_blocked(&rules, Some("chrome.exe"), "随便什么内容"));
    }

    #[test]
    fn 未知规则类型被忽略() {
        let rules = vec![rule("unknown", "x")];
        assert!(!is_blocked(&rules, Some("chrome.exe"), "x"));
    }

    #[test]
    fn 无规则时一律放行() {
        assert!(!is_blocked(&[], Some("chrome.exe"), "任意内容"));
    }
}
