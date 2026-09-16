use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::storage::db::Db;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PanelPosition {
    Cursor,
    Remember,
}

pub const MAX_ITEMS_RANGE: (i64, i64) = (100, 10_000);
pub const RETENTION_DAYS_RANGE: (i64, i64) = (1, 365);
pub const IMAGE_QUOTA_MB_RANGE: (i64, i64) = (50, 10_240);
pub const HIDE_DELAY_MS_RANGE: (i64, i64) = (0, 5_000);
pub const PANEL_W_RANGE: (i64, i64) = (320, 3_000);
pub const PANEL_H_RANGE: (i64, i64) = (200, 2_000);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub language: String,
    pub autostart: bool,
    pub hotkey: String,
    pub plain_hotkey: String,

    pub max_items: i64,
    pub retention_days: i64,
    pub image_quota_mb: i64,

    pub hide_on_blur: bool,
    pub hide_delay_ms: i64,
    pub panel_position: PanelPosition,
    pub panel_x: Option<i32>,
    pub panel_y: Option<i32>,
    pub panel_w: Option<i64>,
    pub panel_h: Option<i64>,
    pub preview_collapsed: bool,
    pub vim_mode: bool,

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
    pub fn sanitize(&mut self) {
        self.max_items = clamp(self.max_items, MAX_ITEMS_RANGE);
        self.retention_days = clamp(self.retention_days, RETENTION_DAYS_RANGE);
        self.image_quota_mb = clamp(self.image_quota_mb, IMAGE_QUOTA_MB_RANGE);
        self.hide_delay_ms = clamp(self.hide_delay_ms, HIDE_DELAY_MS_RANGE);

        self.panel_w = self.panel_w.map(|v| clamp(v, PANEL_W_RANGE));
        self.panel_h = self.panel_h.map(|v| clamp(v, PANEL_H_RANGE));

        self.language = self.language.trim().to_string();
        if self.language.is_empty() {
            self.language = "system".into();
        }
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

pub fn load(db: &Db) -> Result<AppSettings> {
    let mut obj = serde_json::Map::new();
    for (k, v) in db.all_settings()? {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&v) {
            obj.insert(k, json);
        }
    }

    let mut settings: AppSettings = serde_json::from_value(serde_json::Value::Object(obj))?;
    settings.sanitize();
    Ok(settings)
}

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

pub fn apply_patch(current: &AppSettings, patch: serde_json::Value) -> Result<AppSettings> {
    let serde_json::Value::Object(patch) = patch else {
        return Err(Error::Other("设置补丁必须是对象".into()));
    };

    let mut base = match serde_json::to_value(current)? {
        serde_json::Value::Object(map) => map,
        _ => return Err(Error::Other("当前设置序列化后不是对象".into())),
    };

    for (key, value) in patch {
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
