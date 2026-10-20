use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use base64::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
    pub created_at: String,
}

pub fn get_vessel_root() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("Vessel");
        }
        dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("Vessel")
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join("Library/Application Support/Vessel");
        }
        dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("Vessel")
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("Vessel")
    }
}

pub fn get_profiles_file() -> PathBuf {
    get_vessel_root().join("profiles.json")
}

pub fn get_profile_dir(id: &str) -> PathBuf {
    get_vessel_root().join("profiles").join(id)
}

pub fn get_icons_dir() -> PathBuf {
    get_vessel_root().join("icons")
}

pub fn sanitize_id(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        format!("container-{}", chrono::Utc::now().timestamp())
    } else {
        trimmed.to_string()
    }
}

pub fn normalize_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed)
    }
}

pub fn load_containers() -> Vec<ContainerConfig> {
    let file = get_profiles_file();
    if !file.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(&file)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

pub fn get_container(id: &str) -> Option<ContainerConfig> {
    let containers = load_containers();
    containers.into_iter().find(|c| c.id.eq_ignore_ascii_case(id))
}

pub fn save_containers(containers: &[ContainerConfig]) -> Result<(), String> {
    let root = get_vessel_root();
    let _ = std::fs::create_dir_all(&root);
    let file = get_profiles_file();
    let json = serde_json::to_string_pretty(containers).map_err(|e| e.to_string())?;
    std::fs::write(&file, json).map_err(|e| format!("Failed to save profiles.json: {e}"))
}

pub async fn fetch_favicon(url_str: &str, id: &str) -> Option<String> {
    let parsed_url = url::Url::parse(url_str).ok()?;
    let host = parsed_url.host_str()?;

    let favicon_url = format!("https://www.google.com/s2/favicons?domain={}&sz=128", host);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .ok()?;

    let resp = client.get(&favicon_url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() {
        return None;
    }

    // Save physical icon file for OS shortcuts
    let icons_dir = get_icons_dir();
    let _ = std::fs::create_dir_all(&icons_dir);
    let icon_file = icons_dir.join(format!("{}.png", id));
    let _ = std::fs::write(&icon_file, &bytes);

    // Return base64 data URL for instant manager rendering
    let b64 = format!("data:image/png;base64,{}", BASE64_STANDARD.encode(&bytes));
    Some(b64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_id() {
        assert_eq!(sanitize_id("Spotify Web!"), "spotify-web");
        assert_eq!(sanitize_id("Chat GPT 4.0"), "chat-gpt-4-0");
        assert_eq!(sanitize_id("---Slack---"), "slack");
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("open.spotify.com"), "https://open.spotify.com");
        assert_eq!(normalize_url("https://chatgpt.com"), "https://chatgpt.com");
        assert_eq!(normalize_url("http://localhost:3000"), "http://localhost:3000");
    }

    #[test]
    fn test_profile_dir_path() {
        let dir = get_profile_dir("spotify");
        assert!(dir.ends_with(PathBuf::from("profiles").join("spotify")));
    }
}
