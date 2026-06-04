use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Mutex,
};
use serde::{Deserialize, Serialize};
use url::Url;

pub const MAX_NOTIFICATIONS_PER_MINUTE: u32 = 10;
pub const WEBVIEW_DESTROY_TIMEOUT_SECONDS: u64 = 600;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VesselNotification {
    pub app_id: String,
    pub title: String,
    pub body: String,
    pub time: String,
}

pub struct AppState {
    pub db_path: PathBuf,
    pub sessions_root: PathBuf,
    pub active_profiles: Mutex<HashMap<String, String>>,
    pub app_sleep_seconds: Mutex<HashMap<String, u64>>,
    pub active_webview: Mutex<Option<String>>,
    pub hibernate_tokens: Mutex<HashMap<String, u64>>,
    pub safe_mode: Mutex<bool>,
    pub notification_rate_limit: Mutex<HashMap<String, (u32, i64)>>,
}

pub fn sanitize_segment(input: &str, fallback: &str) -> String {
    let cleaned = input
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

pub fn parse_external_url(raw: &str) -> Result<Url, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty url".to_string());
    }

    Url::parse(trimmed)
        .or_else(|_| Url::parse(&format!("https://{trimmed}")))
        .map_err(|_| format!("invalid url: {trimmed}"))
}
