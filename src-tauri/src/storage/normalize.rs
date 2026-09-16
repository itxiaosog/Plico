use sha2::{Digest, Sha256};

pub fn normalize_text(raw: &str) -> String {
    let unified = raw.replace("\r\n", "\n").replace('\r', "\n");
    unified.trim().to_string()
}

pub fn hash_text(normalized: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex(&hasher.finalize())
}

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
