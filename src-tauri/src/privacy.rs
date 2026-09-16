//! 隐私过滤（F10）：命中规则的内容不入库。
//!
//! 两类规则：
//!   - `app`     —— 按前台进程名排除（如 `1Password.exe`）
//!   - `content` —— 按正则排除（如 16 位银行卡号）
//!
//! 规则每次复制时现查现用。剪贴板事件是人类节奏（每秒最多几次），这点查询
//! 开销可以忽略；换来的是「改完规则立刻生效」，不用维护缓存失效逻辑。

use regex::Regex;

use crate::model::Rule;

/// 命中任一启用规则即返回 `true`，调用方应跳过入库。
pub fn is_blocked(rules: &[Rule], source_app: Option<&str>, text: &str) -> bool {
    rules.iter().any(|rule| match rule.kind.as_str() {
        Rule::KIND_APP => matches_app(&rule.value, source_app),
        Rule::KIND_CONTENT => matches_content(&rule.value, text),
        _ => false,
    })
}

/// 进程名比较：忽略大小写，并统一去掉 `.exe` 后缀。
/// 用户填 `1Password` 还是 `1password.exe` 都能命中。
fn matches_app(rule_value: &str, source_app: Option<&str>) -> bool {
    let Some(source) = source_app else {
        // 拿不到来源进程时不误杀 —— 宁可多记一条，也不要静默丢内容
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

/// 正则匹配。规则写错了（编译不过）就当作不命中 —— 不能让一条坏规则
/// 把整个剪贴板监听卡死。
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
