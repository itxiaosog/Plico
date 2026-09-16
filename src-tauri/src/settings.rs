//! 应用设置（F22）。持久化在 SQLite 的 `settings` 表里，key-value 形式。
//!
//! 读取策略是「默认值 ← 数据库覆盖」的合并：库里没有的 key 自然回落到
//! 默认值，所以**新增设置项不需要写迁移**，老库直接兼容。
//!
//! 写入时前端传的是**补丁**（只带改动的字段），Rust 侧合并后整份落库。

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::storage::db::Db;

/// 面板位置模式（F5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PanelPosition {
    /// 跟随光标：出现在鼠标所在那块屏幕的水平居中、垂直偏上位置。
    Cursor,
    /// 记忆位置：恢复到上次隐藏时的坐标。
    Remember,
}

/// 各项的取值范围。放在 Rust 侧做钳制，前端控件就可以写得宽松些。
pub const MAX_ITEMS_RANGE: (i64, i64) = (100, 10_000);
pub const RETENTION_DAYS_RANGE: (i64, i64) = (1, 365);
pub const IMAGE_QUOTA_MB_RANGE: (i64, i64) = (50, 10_240);
pub const HIDE_DELAY_MS_RANGE: (i64, i64) = (0, 5_000);
/// 面板尺寸（逻辑像素）。下限取「列表区最小宽度 320」—— 预览区折叠时
/// 列表独占窗口，这就是窗口能有多窄。展开时的下限更高，由
/// `panel::apply_min_size` 动态设给窗口。
pub const PANEL_W_RANGE: (i64, i64) = (320, 3_000);
pub const PANEL_H_RANGE: (i64, i64) = (200, 2_000);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    /// 界面语言（system / zh / en）。
    pub language: String,
    pub autostart: bool,
    /// 唤起面板的全局热键，展示用描述文本。
    pub hotkey: String,
    /// 「粘贴为纯文本」的全局热键（F15）。
    pub plain_hotkey: String,

    pub max_items: i64,
    pub retention_days: i64,
    pub image_quota_mb: i64,

    pub hide_on_blur: bool,
    pub hide_delay_ms: i64,
    pub panel_position: PanelPosition,
    /// 面板记忆坐标，仅 `panel_position = remember` 时使用。
    ///
    /// 存**物理像素**：Windows 的虚拟桌面坐标本身就是物理像素，跨屏恢复时
    /// 要的就是这个数。
    pub panel_x: Option<i32>,
    pub panel_y: Option<i32>,
    /// 面板记忆尺寸（**逻辑像素**）。和坐标不同，尺寸是「视觉上的大小」，
    /// 存逻辑值才能在缩放比例不同的显示器之间搬动时保持观感一致。
    pub panel_w: Option<i64>,
    pub panel_h: Option<i64>,
    /// 预览区是否折叠（F5）。折叠后列表区撑满整个窗口。
    pub preview_collapsed: bool,
    /// 面板内用 J / K 移动选中（F5，默认关）。
    pub vim_mode: bool,

    /// F21「粘贴后恢复剪贴板」，2.0 才生效。
    pub restore_clipboard: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "system".into(),
            autostart: false,
            hotkey: "Ctrl+Shift+V".into(),
            plain_hotkey: "Ctrl+Shift+Alt+V".into(),

            max_items: 1000,
            retention_days: 30,
            image_quota_mb: 500,

            hide_on_blur: true,
            hide_delay_ms: 150,
            panel_position: PanelPosition::Cursor,
            panel_x: None,
            panel_y: None,
            panel_w: None,
            panel_h: None,
            preview_collapsed: false,
            vim_mode: false,

            restore_clipboard: false,
        }
    }
}

impl AppSettings {
    /// 把越界的值拉回合法区间。热键与语言只做去空白。
    pub fn sanitize(&mut self) {
        self.max_items = clamp(self.max_items, MAX_ITEMS_RANGE);
        self.retention_days = clamp(self.retention_days, RETENTION_DAYS_RANGE);
        self.image_quota_mb = clamp(self.image_quota_mb, IMAGE_QUOTA_MB_RANGE);
        self.hide_delay_ms = clamp(self.hide_delay_ms, HIDE_DELAY_MS_RANGE);

        // 尺寸是可选的：`None` 表示「没记忆过」，交给窗口默认值。
        // 但记忆过的值可能来自分辨率更小的屏幕，所以要钳制。
        self.panel_w = self.panel_w.map(|v| clamp(v, PANEL_W_RANGE));
        self.panel_h = self.panel_h.map(|v| clamp(v, PANEL_H_RANGE));

        self.language = self.language.trim().to_string();
        if self.language.is_empty() {
            self.language = "system".into();
        }
        // 只允许这三个值；其他值静默回 system，不给 UI 层塞脏数据
        if !matches!(self.language.as_str(), "system" | "zh" | "en") {
            self.language = "system".into();
        }

        self.hotkey = self.hotkey.trim().to_string();
        if self.hotkey.is_empty() {
            self.hotkey = AppSettings::default().hotkey;
        }
        self.plain_hotkey = self.plain_hotkey.trim().to_string();
        if self.plain_hotkey.is_empty() {
            self.plain_hotkey = AppSettings::default().plain_hotkey;
        }
    }
}

fn clamp(v: i64, (lo, hi): (i64, i64)) -> i64 {
    v.clamp(lo, hi)
}

/// 从 `settings` 表读全量设置。坏值（解析不出 JSON）直接忽略，回落到默认值，
/// 不让一条脏数据把整个设置页打挂。
pub fn load(db: &Db) -> Result<AppSettings> {
    let mut obj = serde_json::Map::new();
    for (k, v) in db.all_settings()? {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&v) {
            obj.insert(k, json);
        }
    }

    // `#[serde(default)]` 挂在结构体上，缺失字段会走 Default，所以这里允许字段不全。
    let mut settings: AppSettings = serde_json::from_value(serde_json::Value::Object(obj))?;
    settings.sanitize();
    Ok(settings)
}

/// 整份落库。每个字段存成一行，值是它的 JSON 文本。
pub fn save(db: &Db, settings: &AppSettings) -> Result<()> {
    let value = serde_json::to_value(settings)?;
    let serde_json::Value::Object(map) = value else {
        return Err(Error::Other("设置序列化后不是对象".into()));
    };

    let pairs: Vec<(String, String)> = map
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();
    db.put_settings(&pairs)
}

/// 把前端传来的补丁合并到当前设置上，再钳制一遍。
pub fn apply_patch(current: &AppSettings, patch: serde_json::Value) -> Result<AppSettings> {
    let serde_json::Value::Object(patch) = patch else {
        return Err(Error::Other("设置补丁必须是对象".into()));
    };

    let mut base = match serde_json::to_value(current)? {
        serde_json::Value::Object(map) => map,
        _ => return Err(Error::Other("当前设置序列化后不是对象".into())),
    };

    for (key, value) in patch {
        // 未知字段直接丢弃，避免脏 key 混进库里
        if base.contains_key(&key) {
            base.insert(key, value);
        }
    }

    let mut merged: AppSettings = serde_json::from_value(serde_json::Value::Object(base))?;
    merged.sanitize();
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 补丁只改指定字段() {
        let current = AppSettings::default();
        let patched = apply_patch(&current, serde_json::json!({ "autostart": true })).unwrap();

        assert!(patched.autostart);
        // 其余字段保持原值
        assert_eq!(patched.max_items, current.max_items);
        assert_eq!(patched.hotkey, current.hotkey);
    }

    #[test]
    fn 未知字段被丢弃() {
        let current = AppSettings::default();
        let patched = apply_patch(&current, serde_json::json!({ "不存在": 1 })).unwrap();
        assert_eq!(patched, current);
    }

    #[test]
    fn 越界值被钳制() {
        let current = AppSettings::default();
        let patched = apply_patch(
            &current,
            serde_json::json!({ "maxItems": 999_999, "retentionDays": 0 }),
        )
        .unwrap();

        assert_eq!(patched.max_items, MAX_ITEMS_RANGE.1);
        assert_eq!(patched.retention_days, RETENTION_DAYS_RANGE.0);
    }
}
