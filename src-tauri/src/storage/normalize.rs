use sha2::{Digest, Sha256};

/// 保守归一化 —— 规则见《功能规格说明书.md》F2。
///
/// 只做两件事，顺序固定：
///   1. 统一换行符：`\r\n` 与 `\r` 全部换成 `\n`
///   2. 去掉首尾空白（空格 / 制表符 / 换行）
///
/// 刻意 **不** 做：大小写转换、连续空白折叠、全角半角转换、Unicode 正规化。
/// 这几类操作会把用户有意区分的内容（`Hello` vs `hello`、代码缩进差异）
/// 误判成同一条，破坏去重的可预期性。
pub fn normalize_text(raw: &str) -> String {
    let unified = raw.replace("\r\n", "\n").replace('\r', "\n");
    unified.trim().to_string()
}

/// 对归一化后的文本求 SHA-256，返回小写十六进制串。
pub fn hash_text(normalized: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex(&hasher.finalize())
}

/// 对任意字节求 SHA-256。图片按 PNG 文件字节哈希，文件列表按排序后路径列表哈希。
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex(&hasher.finalize())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 与 `monitor::ingest` 的调用顺序一致：先归一化，再哈希。
    ///
    /// 刻意不提供 `hash_normalized(raw)` 这种一步到位的包装 —— 生产路径需要
    /// 归一化后的文本本身（要存进 `plain_text`），所以只能归一化一次、再哈希。
    /// 有了包装函数，早晚会有人在热路径上写 `hash_normalized` + `normalize_text`
    /// 而白白归一化两遍。
    fn hash_of(raw: &str) -> String {
        hash_text(&normalize_text(raw))
    }

    #[test]
    fn 统一换行符() {
        assert_eq!(normalize_text("a\r\nb\rc"), "a\nb\nc");
    }

    #[test]
    fn 去首尾空白() {
        assert_eq!(normalize_text("  hello\n"), "hello");
        assert_eq!(normalize_text("\t x \t"), "x");
    }

    #[test]
    fn 大小写敏感_这是刻意的() {
        assert_ne!(hash_of("hello"), hash_of("Hello"));
    }

    #[test]
    fn 空白差异视为同一条() {
        assert_eq!(hash_of("  hello\r\n"), hash_of("hello\n"));
    }

    #[test]
    fn 内部空白不折叠() {
        assert_ne!(hash_of("a  b"), hash_of("a b"));
    }
}
