use serde::{Deserialize, Serialize};

/// 条目类型。对应规格书 items.type 列的取值域。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Text,
    RichText,
    Image,
    Files,
    Link,
    Color,
}

impl ItemType {
    pub fn as_str(self) -> &'static str {
        match self {
            ItemType::Text => "text",
            ItemType::RichText => "rich_text",
            ItemType::Image => "image",
            ItemType::Files => "files",
            ItemType::Link => "link",
            ItemType::Color => "color",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "text" => ItemType::Text,
            "rich_text" => ItemType::RichText,
            "image" => ItemType::Image,
            "files" => ItemType::Files,
            "link" => ItemType::Link,
            "color" => ItemType::Color,
            _ => return None,
        })
    }
}

/// 落库后的完整条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: ItemType,
    pub content: Option<String>,
    pub html_content: Option<String>,
    pub plain_text: Option<String>,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub content_hash: String,
    pub source_app: Option<String>,
    pub source_title: Option<String>,
    pub group_id: Option<i64>,
    pub pinned: bool,
    pub created_at: i64,
    pub last_copied_at: i64,
    pub last_used_at: Option<i64>,
}

/// 待入库的新条目。hash 由调用方按 F2 规则算好后传入。
#[derive(Debug, Clone, Default)]
pub struct NewItem {
    pub kind: ItemType,
    pub content: Option<String>,
    pub html_content: Option<String>,
    pub plain_text: Option<String>,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub source_app: Option<String>,
    pub source_title: Option<String>,
}

impl Default for ItemType {
    fn default() -> Self {
        ItemType::Text
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag { pub id: i64, pub name: String }

/// 标签 + 使用计数。
///
/// 设置窗口的标签页需要知道「这个标签被多少条记录用着」—— 决定合并还是删除时
/// 这是必要信息，否则用户只能盲操作。单独开一个结构而不是给 `Tag` 加字段：
/// `Tag` 还用在条目行内展示上，那里不需要计数，加了会平白多一次 JOIN。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagStat { pub id: i64, pub name: String, pub count: i64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet { pub id:i64, pub title:String, pub content:String, pub tags:Option<String>, pub shortcut:Option<String>, pub created_at:i64, pub updated_at:i64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSnippet { pub title: String, pub content: String, pub tags: Option<String>, pub shortcut: Option<String> }


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total: i64,
    pub pinned: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: i64,
    pub kind: String,
    pub value: String,
    pub enabled: bool,
}

impl Rule {
    pub const KIND_APP: &'static str = "app";
    pub const KIND_CONTENT: &'static str = "content";

    pub fn is_valid_kind(kind: &str) -> bool {
        kind == Self::KIND_APP || kind == Self::KIND_CONTENT
    }
}
