#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPinState {
    pub is_pinned: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPlacementState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub display_id: Option<String>,
    pub updated_at: String,
}
