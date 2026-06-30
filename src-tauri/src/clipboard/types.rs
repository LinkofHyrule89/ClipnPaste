use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClipItemType {
    Text,
    Image,
}

impl ClipItemType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "image" => Self::Image,
            _ => Self::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: String,
    pub item_type: ClipItemType,
    pub preview: String,
    pub content: String,
    pub pinned: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipItemSummary {
    pub id: String,
    pub item_type: ClipItemType,
    pub preview: String,
    pub pinned: bool,
    pub created_at: i64,
}

impl From<ClipItem> for ClipItemSummary {
    fn from(item: ClipItem) -> Self {
        Self {
            id: item.id,
            item_type: item.item_type,
            preview: item.preview,
            pinned: item.pinned,
            created_at: item.created_at,
        }
    }
}