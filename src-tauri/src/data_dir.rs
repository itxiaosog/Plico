use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::error::{Error, Result};

pub fn resolve(app: &tauri::AppHandle) -> Result<PathBuf> {
    let preferred = install_dir()?.join("data");
    if is_writable(&preferred) {
        return Ok(preferred);
    }

    let fallback = legacy_dir(app)?;
    eprintln!(
        "[plico] 安装目录不可写（{}），数据改放：{}",
        preferred.display(),
        fallback.display()
    );
    Ok(fallback)
}

pub fn install_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().map_err(|e| Error::Other(format!("取不到程序路径：{e}")))?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::Other("程序路径没有父目录".into()))
}

pub fn legacy_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| Error::Other(format!("取不到数据目录：{e}")))
}

fn is_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".plico-write-probe");
    match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

pub fn migrate(from: &Path, to: &Path) -> Result<bool> {
    let from_db = from.join("plico.db");
    let to_db = to.join("plico.db");

    if from == to || to.starts_with(from) || !from_db.exists() || to_db.exists() {
        return Ok(false);
    }

    std::fs::create_dir_all(to)?;

    for entry in ["plico.db", "plico.db-wal", "plico.db-shm"] {
        let src = from.join(entry);
        if src.exists() {
            std::fs::copy(&src, to.join(entry))
                .map_err(|e| Error::Other(format!("迁移 {entry} 失败：{e}")))?;
        }
    }
    for dir in ["images", "thumbs"] {
        let src = from.join(dir);
        if src.is_dir() {
            copy_dir(&src, &to.join(dir))?;
        }
    }

    println!(
        "[plico] 数据目录已迁移：{} → {}",
        from.display(),
        to.display()
    );
    Ok(true)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("plico-dd-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn 临时目录可写() {
        let dir = temp_dir("w");
        assert!(is_writable(&dir));
        assert!(!dir.join(".plico-write-probe").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 目录不存在时会先建再判可写() {
        let dir = temp_dir("w2").join("nested");
        assert!(!dir.exists());
        assert!(is_writable(&dir));
        assert!(dir.is_dir());
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn 目标已有库时不覆盖() {
        let from = temp_dir("from");
        let to = temp_dir("to");
        std::fs::write(from.join("plico.db"), b"old").unwrap();
        std::fs::write(to.join("plico.db"), b"existing").unwrap();

        assert!(!migrate(&from, &to).unwrap());
        assert_eq!(std::fs::read(to.join("plico.db")).unwrap(), b"existing");

        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_dir_all(&to);
    }

    #[test]
    fn 真的迁移() {
        let from = temp_dir("from2");
        let to = temp_dir("to2");
        std::fs::write(from.join("plico.db"), b"db").unwrap();
        std::fs::write(from.join("plico.db-wal"), b"wal").unwrap();
        std::fs::create_dir_all(from.join("images")).unwrap();
        std::fs::write(from.join("images/a.png"), b"png").unwrap();

        assert!(migrate(&from, &to).unwrap());
        assert_eq!(std::fs::read(to.join("plico.db")).unwrap(), b"db");
        assert_eq!(std::fs::read(to.join("plico.db-wal")).unwrap(), b"wal");
        assert_eq!(std::fs::read(to.join("images/a.png")).unwrap(), b"png");
        assert!(from.join("plico.db").exists());

        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_dir_all(&to);
    }

    #[test]
    fn 目标是源的子目录时不搬() {
        let from = temp_dir("from3");
        let to = from.join("nested");
        std::fs::write(from.join("plico.db"), b"db").unwrap();

        assert!(!migrate(&from, &to).unwrap());
        assert!(!to.join("plico.db").exists());

        let _ = std::fs::remove_dir_all(&from);
    }

    #[test]
    fn 源没库可迁() {
        let from = temp_dir("from4");
        let to = temp_dir("to4");
        assert!(!migrate(&from, &to).unwrap());
        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_dir_all(&to);
    }
}
