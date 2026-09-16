use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::clipboard::{classify, files, image};
use crate::model::{ItemType, NewItem};
use crate::platform;
use crate::privacy;
use crate::settings::AppSettings;
use crate::storage::db::Db;
use crate::storage::normalize;

const POLL_INTERVAL_MS: u64 = 300;
const CATCHUP_INTERVAL_MS: u64 = 100;
const MAX_CATCHUP: u32 = 3;

const CLEANUP_INTERVAL_MS: i64 = 6 * 60 * 60 * 1000;

const RESTART_DELAY_MS: u64 = 5_000;

const SUPPRESS_MS: i64 = 700;

pub const EVENT_ITEMS_CHANGED: &str = "plico://items-changed";

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug)]
pub struct Suppress {
    until_ms: AtomicI64,
}

impl Suppress {
    pub fn new() -> Self {
        Self {
            until_ms: AtomicI64::new(0),
        }
    }

    pub fn arm(&self, ms: i64) {
        self.until_ms.store(now_ms() + ms, Ordering::SeqCst);
    }

    pub fn arm_default(&self) {
        self.arm(SUPPRESS_MS);
    }

    pub fn is_armed(&self) -> bool {
        now_ms() < self.until_ms.load(Ordering::SeqCst)
    }
}

impl Default for Suppress {
    fn default() -> Self {
        Self::new()
    }
}

pub fn spawn(
    app: AppHandle,
    db: Arc<Db>,
    suppress: Arc<Suppress>,
    settings: Arc<RwLock<AppSettings>>,
) {
    std::thread::Builder::new()
        .name("plico-clipboard-monitor".into())
        .spawn(move || loop {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                watch_loop(&app, &db, &suppress, &settings);
            }));

            match outcome {
                Ok(()) => eprintln!(
                    "[plico] 剪贴板监听已停止，{RESTART_DELAY_MS}ms 后重试"
                ),
                Err(payload) => eprintln!(
                    "[plico] 剪贴板监听崩溃：{}，{RESTART_DELAY_MS}ms 后重启",
                    panic_message(&payload)
                ),
            }

            std::thread::sleep(Duration::from_millis(RESTART_DELAY_MS));
        })
        .expect("无法启动剪贴板监听线程");
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "未知原因".to_string()
    }
}

fn watch_loop(
    app: &AppHandle,
    db: &Db,
    suppress: &Suppress,
    settings: &RwLock<AppSettings>,
) {
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[plico] 剪贴板初始化失败：{e}");
            return;
        }
    };

    let mut last_seq = platform::clipboard_sequence();
    let mut last_cleanup = now_ms();

    loop {
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));

        let now = now_ms();
        if now - last_cleanup >= CLEANUP_INTERVAL_MS {
            last_cleanup = now;
            run_cleanup(db, settings, now);
        }

        let current = platform::clipboard_sequence();
        if current == last_seq {
            continue;
        }

        let mut stable = current;
        for _ in 0..MAX_CATCHUP {
            std::thread::sleep(Duration::from_millis(CATCHUP_INTERVAL_MS));
            let s = platform::clipboard_sequence();
            if s == stable {
                break;
            }
            stable = s;
        }
        last_seq = stable;

        if suppress.is_armed() {
            continue;
        }

        let snapshot = read_settings(settings);
        let (exe, title) = platform::foreground_app();

        let outcome = if let Ok(text) = clipboard.get_text() {
            if text.trim().is_empty() {
                Ok(IngestOutcome::Skipped)
            } else if is_privately_blocked(db, exe.as_deref(), &text) {
                Ok(IngestOutcome::Skipped)
            } else {
                let html = clipboard.get().html().ok();
                ingest_text(db, &text, html.as_deref(), exe, title, snapshot.max_items)
            }
        } else if let Ok(img) = clipboard.get_image() {
            if is_privately_blocked(db, exe.as_deref(), "") {
                Ok(IngestOutcome::Skipped)
            } else {
                ingest_image(app, db, &img, exe, title, snapshot.max_items)
            }
        } else if let Some(paths) = files::read(&mut clipboard) {
            if is_privately_blocked(db, exe.as_deref(), "") {
                Ok(IngestOutcome::Skipped)
            } else {
                ingest_files(db, &paths, exe, title, snapshot.max_items)
            }
        } else {
            Ok(IngestOutcome::Skipped)
        };

        match outcome {
            Ok(IngestOutcome::Inserted) | Ok(IngestOutcome::Merged) => {
                let _ = app.emit(EVENT_ITEMS_CHANGED, ());
            }
            Ok(IngestOutcome::Skipped) => {}
            Err(e) => eprintln!("[plico] 入库失败：{e}"),
        }
    }
}

fn read_settings(settings: &RwLock<AppSettings>) -> AppSettings {
    settings
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn is_privately_blocked(db: &Db, source_app: Option<&str>, text: &str) -> bool {
    match db.enabled_rules() {
        Ok(rules) => privacy::is_blocked(&rules, source_app, text),
        Err(e) => {
            eprintln!("[plico] 读取隐私规则失败，本条按不命中处理：{e}");
            false
        }
    }
}

fn run_cleanup(db: &Db, settings: &RwLock<AppSettings>, now: i64) {
    let snapshot = read_settings(settings);
    match db.purge_expired(snapshot.retention_days, now) {
        Ok(0) => {}
        Ok(n) => println!("[plico] 保留策略清理：删除 {n} 条过期记录"),
        Err(e) => eprintln!("[plico] 保留策略清理失败：{e}"),
    }

    match db.purge_images(snapshot.image_quota_mb) {
        Ok(0) => {}
        Ok(n) => println!("[plico] 图片配额清理：删除 {n} 条记录"),
        Err(e) => eprintln!("[plico] 图片配额清理失败：{e}"),
    }
}

enum IngestOutcome {
    Inserted,
    Merged,
    Skipped,
}

fn ingest_text(
    db: &Db,
    raw: &str,
    html: Option<&str>,
    source_app: Option<String>,
    source_title: Option<String>,
    max_items: i64,
) -> crate::error::Result<IngestOutcome> {
    let plain = normalize::normalize_text(raw);
    let hash = normalize::hash_text(&plain);

    let kind = if html.is_some() {
        ItemType::RichText
    } else {
        classify(raw)
    };

    let new = NewItem {
        kind,
        content: Some(plain.clone()),
        html_content: html.map(str::to_string),
        plain_text: Some(plain),
        image_path: None,
        thumb_path: None,
        source_app,
        source_title,
    };

    let (_, merged) = db.upsert(&new, &hash, now_ms(), max_items)?;
    Ok(if merged {
        IngestOutcome::Merged
    } else {
        IngestOutcome::Inserted
    })
}

fn ingest_image(
    app: &AppHandle,
    db: &Db,
    img: &arboard::ImageData<'_>,
    source_app: Option<String>,
    source_title: Option<String>,
    max_items: i64,
) -> crate::error::Result<IngestOutcome> {
    use tauri::Manager;

    let png = image::encode_png_pub(img)?;
    let hash = normalize::hash_bytes(&png);

    if db.get_by_hash_pub(&hash)?.is_some() {
        let new = NewItem {
            kind: ItemType::Image,
            content: None,
            html_content: None,
            plain_text: None,
            image_path: None,
            thumb_path: None,
            source_app,
            source_title,
        };
        let (_, merged) = db.upsert(&new, &hash, now_ms(), max_items)?;
        return Ok(if merged {
            IngestOutcome::Merged
        } else {
            IngestOutcome::Inserted
        });
    }

    let data_dir = app.state::<crate::AppState>().data_dir.clone();

    let (image_path, thumb_path) = image::save_image_bytes(&data_dir, img, &png, &hash)?;

    let name = image_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "image.png".to_string());

    let new = NewItem {
        kind: ItemType::Image,
        content: Some(name.clone()),
        html_content: None,
        plain_text: Some(name),
        image_path: Some(image_path.to_string_lossy().to_string()),
        thumb_path: Some(thumb_path.to_string_lossy().to_string()),
        source_app,
        source_title,
    };

    let (_, merged) = db.upsert(&new, &hash, now_ms(), max_items)?;
    Ok(if merged {
        IngestOutcome::Merged
    } else {
        IngestOutcome::Inserted
    })
}

fn ingest_files(
    db: &Db,
    paths: &[PathBuf],
    source_app: Option<String>,
    source_title: Option<String>,
    max_items: i64,
) -> crate::error::Result<IngestOutcome> {
    let Some((content, plain, hash)) = files::ingest(paths) else {
        return Ok(IngestOutcome::Skipped);
    };

    let new = NewItem {
        kind: ItemType::Files,
        content: Some(content),
        html_content: None,
        plain_text: Some(plain),
        image_path: None,
        thumb_path: None,
        source_app,
        source_title,
    };

    let (_, merged) = db.upsert(&new, &hash, now_ms(), max_items)?;
    Ok(if merged {
        IngestOutcome::Merged
    } else {
        IngestOutcome::Inserted
    })
}
