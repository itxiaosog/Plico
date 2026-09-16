//! 数据目录的解析与旧数据搬迁。
//!
//! 数据固定放在**安装目录**下的 `data/`（和 `Plico.exe` 同级）：可见、好备份、
//! 拷走整个安装目录就等于搬家。代价有两条，是这个选择的固有属性：
//!
//!   1. **卸载会连数据一起删** —— NSIS 卸载时删安装目录，剪贴板历史跟着走；
//!   2. **同机多用户共用一份数据** —— 安装目录是全局的，不按用户隔离。
//!
//! 两个例外：
//!
//!   - **安装目录不可写**（比如装到了 `C:\Program Files` 又用标准用户跑）——
//!     回退到 `%APPDATA%/com.plico.app/`。不回退的话连库都开不了，应用直接
//!     起不来，比「数据位置不如预期」严重得多；
//!   - **老版本的数据在 `%APPDATA%`** —— 启动时做一次性搬迁（见 `migrate`），
//!     原目录保留当备份。
//!
//! 1.x 允许用户在设置里自定义数据目录，这个开关**已经取消**：`resolve` 不再
//! 读设置，`AppSettings` 里也没有 `data_dir` 字段了。

use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::error::{Error, Result};

/// 该用的数据目录：安装目录下的 `data/`，不可写时回退到 `%APPDATA%`。
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

/// 安装目录 —— 当前可执行文件所在的那个目录。
pub fn install_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().map_err(|e| Error::Other(format!("取不到程序路径：{e}")))?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::Other("程序路径没有父目录".into()))
}

/// 1.x 的系统默认数据目录（`%APPDATA%/com.plico.app/`）。
///
/// 只在「安装目录不可写的回退」和「搬迁老数据」两个地方用 —— 正常启动的
/// 数据目录不是它。
pub fn legacy_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| Error::Other(format!("取不到数据目录：{e}")))
}

/// 目录能不能写。
///
/// 光看 `create_dir_all` 不够 —— `C:\Program Files` 下**已存在**的目录，
/// 标准用户 `create_dir_all` 会返回 Ok，真正写文件时才报错。所以这里实打实
/// 写一个探针文件再删掉。一次调用只花几十微秒，且只在启动与少数命令里跑。
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

/// 把数据从 `from` 迁移到 `to`。启动时用来把 1.x 的 `%APPDATA%` 数据搬到
/// 安装目录，幂等 —— 重复启动、搬一半重来都安全。
///
/// 满足以下任一条件就什么都不做：
///   - `from == to`（或 to 是 from 的子目录，那说明目标指到了自己里面，
///     搬进去会造成无限嵌套）
///   - `to` 已经有 `plico.db`（说明之前已经迁过）
///   - `from` 不存在或没有 `plico.db`（没有可迁的东西）
///
/// 返回 true 表示真的搬了。
pub fn migrate(from: &Path, to: &Path) -> Result<bool> {
    let from_db = from.join("plico.db");
    let to_db = to.join("plico.db");

    if from == to || to.starts_with(from) || !from_db.exists() || to_db.exists() {
        return Ok(false);
    }

    std::fs::create_dir_all(to)?;

    // 只搬认识的东西：库 + WAL/SHM + 图片目录。其他杂物（日志、临时文件）留在原处。
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
        // 探针文件用完就删，不在数据目录里留垃圾
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
        // 原目录不动
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
