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

/// 轮询间隔（F1）。Windows 用 GetClipboardSequenceNumber 比对比内容便宜得多。
const POLL_INTERVAL_MS: u64 = 300;
/// 检测到变化后的追读间隔，用来避开「复制到一半」的中间态。
const CATCHUP_INTERVAL_MS: u64 = 100;
/// 追读次数上限，防止脚本疯狂写剪贴板时死循环。
const MAX_CATCHUP: u32 = 3;

/// 保留策略的后台执行周期（F14）：规格要求「启动时 + 每 6 小时」。
const CLEANUP_INTERVAL_MS: i64 = 6 * 60 * 60 * 1000;

/// 监听线程崩溃后的重启间隔（规格书第 7 节）。
const RESTART_DELAY_MS: u64 = 5_000;

/// 自身写入抑制窗口时长。
///
/// 规格书 6.2 写的是 500ms，这里上调到 700ms —— 理由是实测出来的：
/// 最坏情况下监听线程要经历「300ms 轮询间隔 + 3×100ms 追读」才走到抑制检查，
/// 也就是写入后约 600ms 才被看到。500ms 的窗口会在这条路径上漏掉自身写入，
/// 导致 Plico 粘贴过的内容被当成一次新的复制重复入库。
/// 700ms = 600ms 最坏检测延迟 + 100ms 余量。
const SUPPRESS_MS: i64 = 700;

pub const EVENT_ITEMS_CHANGED: &str = "plico://items-changed";

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 「抑制窗口」：Plico 自己写剪贴板后的这段时间内，监听器不记录变化（规格书 6.2）。
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

    /// 开启抑制窗口。在写入剪贴板 **之前** 调用。
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

/// 启动监听线程。
///
/// 外层是一个**监督循环**：规格书第 7 节要求「监听线程崩溃自恢复（panic 后 5s
/// 重启）」。这也是 `[profile.release]` 不能开 `panic = "abort"` 的原因 ——
/// abort 直接终止进程，线程级恢复无从谈起。
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
                // watch_loop 正常返回 = 剪贴板初始化失败。等一会儿再试，
                // 而不是让监听永久失效（比如另一程序短暂占着剪贴板时）。
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

/// panic payload 转成人话。默认的 payload 只会在日志里打成一坨 `Any`。
fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "未知原因".to_string()
    }
}

/// 监听主循环。正常情况下永远不返回。
fn watch_loop(
    app: &AppHandle,
    db: &Db,
    suppress: &Suppress,
    settings: &RwLock<AppSettings>,
) {
    // arboard 的 Clipboard 在 Windows 上会绑定创建它的线程，必须在监听线程内创建。
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
        // 低频保留策略清理（F14）。放在轮询循环里而不是另起线程：
        // 一次 SQL DELETE 而已，没必要为它多养一个常驻线程。
        if now - last_cleanup >= CLEANUP_INTERVAL_MS {
            last_cleanup = now;
            run_cleanup(db, settings, now);
        }

        let current = platform::clipboard_sequence();
        if current == last_seq {
            continue;
        }

        // 追读：等剪贴板稳定下来，避免读到中间态
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

        // 自身写入，跳过
        if suppress.is_armed() {
            continue;
        }

        let snapshot = read_settings(settings);
        let (exe, title) = platform::foreground_app();

        // 剪贴板一次只表达一种主内容。读取顺序：文本 → 图片 → 文件。
        // Windows 上图片/文件剪贴板经常不带文本格式；文本最先读是因为
        // 它最便宜，且占实际复制的绝大多数。
        let outcome = if let Ok(text) = clipboard.get_text() {
            if text.trim().is_empty() {
                Ok(IngestOutcome::Skipped)
            } else if is_privately_blocked(db, exe.as_deref(), &text) {
                // 隐私过滤（F10）：命中规则的内容不入库
                Ok(IngestOutcome::Skipped)
            } else {
                // 同一剪贴板可能同时带 HTML 格式（CF_HTML）。顺带读一下，
                // 读不到就当普通文本 —— 富文本副本是锦上添花，不该影响入库。
                let html = clipboard.get().html().ok();
                ingest_text(db, &text, html.as_deref(), exe, title, snapshot.max_items)
            }
        } else if let Ok(img) = clipboard.get_image() {
            // 图片没有「内容正则」可匹，但来源应用排除（F10）依然要过一遍
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

/// 读一份设置快照。锁中毒时取内层值继续用 —— 设置是只读快照，不会出现半更新状态。
fn read_settings(settings: &RwLock<AppSettings>) -> AppSettings {
    settings
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// 查一次隐私规则并判定。规则读不出来时**放行**：宁可多记一条，
/// 也不要因为一次数据库抖动把用户的内容静默丢掉。
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

    // 图片总容量（F14）：超配额时淘汰最旧的未置顶图片条目
    match db.purge_images(snapshot.image_quota_mb) {
        Ok(0) => {}
        Ok(n) => println!("[plico] 图片配额清理：删除 {n} 条记录"),
        Err(e) => eprintln!("[plico] 图片配额清理失败：{e}"),
    }
}

/// 一次剪贴板变化的处理结果。用于决定要不要通知前端刷新。
enum IngestOutcome {
    /// 新条目入库
    Inserted,
    /// 命中去重，老条目被顶到最前
    Merged,
    /// 主动跳过（空内容 / 隐私规则 / 不认识的类型）
    Skipped,
}

/// 归一化 → 哈希 → 入库。文本类条目的 hash 基于 plain_text（F2）。
///
/// 容量上限由调用方从设置里带进来（F14），不在这里读库。
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

    // hash 永远按 plain_text 算（F2）：同一段文字以富文本或纯文本复制
    // 应合并成同一条，类型以「这次剪贴板带没带 HTML」为准。
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

/// 图片（F12）：PNG 原图 + 缩略图落盘，路径入库，hash 按 PNG 字节算。
///
/// 落盘依赖数据目录，所以要比文本多拿一个 `AppHandle` —— 目录从 `AppState`
/// 取启动时解析好的那一份，不在这里另算。
fn ingest_image(
    app: &AppHandle,
    db: &Db,
    img: &arboard::ImageData<'_>,
    source_app: Option<String>,
    source_title: Option<String>,
    max_items: i64,
) -> crate::error::Result<IngestOutcome> {
    use tauri::Manager;

    // 原样编码成 PNG 再哈希：同一个位图不管从哪个应用复制出来，
    // 编码结果都一样，去重才有意义。
    let png = image::encode_png_pub(img)?;
    let hash = normalize::hash_bytes(&png);

    // 命中去重时不必再写一遍文件 —— 文件内容按哈希命名，已经在那了。
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

    // 数据目录从全局状态取。这里曾经写死 `app_data_dir()`，于是用户改过数据
    // 目录之后「库在新目录、图片还在 %APPDATA%」，图片预览整片空白。
    let data_dir = app.state::<crate::AppState>().data_dir.clone();

    // save_image 内部会再编码一次 PNG，把刚才那份字节传下去避免二次编码。
    let (image_path, thumb_path) = image::save_image_bytes(&data_dir, img, &png, &hash)?;

    let name = image_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "image.png".to_string());

    let new = NewItem {
        kind: ItemType::Image,
        content: Some(name.clone()),
        html_content: None,
        // plain_text 存文件名，搜索能命中（F6）；原图/缩略图路径给列表和预览用
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

/// 文件列表（F13）：`content` 存 JSON 路径数组，`plain_text` 存文件名列表，
/// hash 按排序后路径算（顺序不同视为同一条）。
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
