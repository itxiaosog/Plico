//! F13 文件：剪贴板文件列表的读、写与内容表达。
//!
//! 读写都走 arboard（Windows 上内部是 CF_HDROP）。这里负责的是
//! 「路径列表 ↔ 入库表达」的转换与校验：
//!   - 入库 `content` 存 JSON 数组（与 mock 数据、前端 `parseFileList` 对齐）；
//!   - `plain_text` 存空格分隔的文件名列表，给 F6 搜索命中文件名用；
//!   - `content_hash` 按**排序后**路径列表计算（F2：顺序不同视为同一条）。

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::storage::normalize;

/// 单次记录的路径数上限（F13：默认 50）。超出时截断 —— 复制一整个目录树
/// 不该把列表塞爆。
pub const MAX_FILES: usize = 50;

/// 从剪贴板读文件列表。读不到或为空都返回 None，由调用方决定要不要继续尝试别的类型。
pub fn read(clipboard: &mut arboard::Clipboard) -> Option<Vec<PathBuf>> {
    clipboard.get().file_list().ok().filter(|v| !v.is_empty())
}

/// 把路径列表写回剪贴板（粘贴方向）。
pub fn write(clipboard: &mut arboard::Clipboard, paths: &[PathBuf]) -> Result<()> {
    clipboard
        .set()
        .file_list(paths)
        .map_err(|e| Error::Clipboard(e))
}

/// 把路径列表转换成入库用的 (content, plain_text, hash)。
///
/// 返回 None 表示这批路径没有可入库的内容（空列表）。
pub fn ingest(paths: &[PathBuf]) -> Option<(String, String, String)> {
    let paths: Vec<String> = paths
        .iter()
        .take(MAX_FILES)
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    if paths.is_empty() {
        return None;
    }

    let content = serde_json::to_string(&paths).ok()?;
    let plain = paths
        .iter()
        .filter_map(|p| Path::new(p).file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(" ");

    // F2：按排序后的路径列表求哈希，「Ctrl+A 复制出来的顺序」不该影响去重
    let mut sorted = paths.clone();
    sorted.sort();
    let hash = normalize::hash_text(&sorted.join("\n"));

    Some((content, plain, hash))
}

/// 解析入库的 `content` 还原路径列表。
pub fn parse(content: &str) -> Vec<PathBuf> {
    serde_json::from_str::<Vec<String>>(content)
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

/// 还有多少路径真实存在于磁盘上。用于「源文件已删除 → 标记失效」（F13）。
pub fn existing_paths(content: &str) -> Vec<PathBuf> {
    parse(content)
        .into_iter()
        .filter(|p| p.exists())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 顺序不同视为同一条() {
        let a = vec![PathBuf::from("C:/a.txt"), PathBuf::from("C:/b.txt")];
        let b = vec![PathBuf::from("C:/b.txt"), PathBuf::from("C:/a.txt")];
        let (_, _, ha) = ingest(&a).unwrap();
        let (_, _, hb) = ingest(&b).unwrap();
        assert_eq!(ha, hb);
    }

    #[test]
    fn content是json_plain是文件名列表() {
        let paths = vec![
            PathBuf::from("D:/工作/report.pdf"),
            PathBuf::from("D:/工作/data.xlsx"),
        ];
        let (content, plain, _) = ingest(&paths).unwrap();
        assert_eq!(
            content,
            r#"["D:/工作/report.pdf","D:/工作/data.xlsx"]"#
        );
        assert_eq!(plain, "report.pdf data.xlsx");
    }

    #[test]
    fn 超过上限被截断() {
        let paths: Vec<PathBuf> = (0..60).map(|i| PathBuf::from(format!("C:/f{i}.txt"))).collect();
        let (content, _, _) = ingest(&paths).unwrap();
        assert_eq!(parse(&content).len(), MAX_FILES);
    }

    #[test]
    fn 空列表不入库() {
        assert!(ingest(&[]).is_none());
    }
}
