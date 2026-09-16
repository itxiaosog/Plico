use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::storage::normalize;

pub const MAX_FILES: usize = 50;

pub fn read(clipboard: &mut arboard::Clipboard) -> Option<Vec<PathBuf>> {
    clipboard.get().file_list().ok().filter(|v| !v.is_empty())
}

pub fn write(clipboard: &mut arboard::Clipboard, paths: &[PathBuf]) -> Result<()> {
    clipboard
        .set()
        .file_list(paths)
        .map_err(|e| Error::Clipboard(e))
}

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

    let mut sorted = paths.clone();
    sorted.sort();
    let hash = normalize::hash_text(&sorted.join("\n"));

    Some((content, plain, hash))
}

pub fn parse(content: &str) -> Vec<PathBuf> {
    serde_json::from_str::<Vec<String>>(content)
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

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
