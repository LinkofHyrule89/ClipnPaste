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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_item_type_roundtrip() {
        assert_eq!(ClipItemType::Text.as_str(), "text");
        assert_eq!(ClipItemType::Image.as_str(), "image");
        assert_eq!(ClipItemType::from_str("image"), ClipItemType::Image);
        assert_eq!(ClipItemType::from_str("text"), ClipItemType::Text);
        assert_eq!(ClipItemType::from_str("other"), ClipItemType::Text);
    }

    #[test]
    fn summary_serde_camel_case() {
        let summary = ClipItemSummary {
            id: "abc".into(),
            item_type: ClipItemType::Image,
            preview: "prev".into(),
            pinned: true,
            created_at: 123,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"itemType\":\"image\""));
        assert!(json.contains("\"createdAt\":123"));
        assert!(json.contains("\"pinned\":true"));
        let back: ClipItemSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "abc");
        assert_eq!(back.item_type, ClipItemType::Image);
        assert_eq!(back.created_at, 123);
    }

    #[test]
    fn summary_from_item() {
        let item = ClipItem {
            id: "1".into(),
            item_type: ClipItemType::Text,
            preview: "p".into(),
            content: "c".into(),
            pinned: false,
            created_at: 9,
        };
        let s = ClipItemSummary::from(item);
        assert_eq!(s.id, "1");
        assert_eq!(s.preview, "p");
        assert!(!s.pinned);
    }
}