use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    Area,
    Screen,
}

impl CaptureMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Area => "area",
            Self::Screen => "screen",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRecord {
    pub id: String,
    pub created_at: String,
    pub file_path: String,
    pub width: u32,
    pub height: u32,
    pub capture_mode: String,
    pub title: String,
    pub description: String,
    pub note: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub ocr_text: String,
    pub enrichment_status: String,
}
