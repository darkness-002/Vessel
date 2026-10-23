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
    #[serde(default)]
    pub icon_path: Option<String>,
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

pub fn get_profile_icon_path(id: &str) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        get_profile_dir(id).join("icon.ico")
    }

    #[cfg(target_os = "macos")]
    {
        get_profile_dir(id).join("icon.png")
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        get_profile_dir(id).join("icon.png")
    }
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

pub fn convert_to_ico(raw_bytes: &[u8]) -> Result<Vec<u8>, String> {
    if raw_bytes.len() >= 4 && raw_bytes[0..4] == [0, 0, 1, 0] {
        return Ok(raw_bytes.to_vec());
    }

    let img = image::load_from_memory(raw_bytes)
        .map_err(|e| format!("Failed to decode image for ICO conversion: {e}"))?;

    let rgba = img.to_rgba8();
    let (orig_w, orig_h) = rgba.dimensions();

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    let standard_sizes = [16u32, 32, 48, 64, 128, 256];
    let max_dim = orig_w.max(orig_h);

    for &size in &standard_sizes {
        if size <= max_dim || size == 32 || size == 48 {
            let resized = if orig_w == size && orig_h == size {
                rgba.clone()
            } else {
                image::imageops::resize(&rgba, size, size, image::imageops::FilterType::Lanczos3)
            };
            let icon_image = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
            let entry = ico::IconDirEntry::encode(&icon_image)
                .map_err(|e| format!("Failed to encode ICO entry ({size}x{size}): {e}"))?;
            icon_dir.add_entry(entry);
        }
    }

    if icon_dir.entries().is_empty() {
        let icon_image = ico::IconImage::from_rgba_data(orig_w, orig_h, rgba.into_raw());
        let entry = ico::IconDirEntry::encode(&icon_image)
            .map_err(|e| format!("Failed to encode fallback ICO entry: {e}"))?;
        icon_dir.add_entry(entry);
    }

    let mut out = Vec::new();
    icon_dir.write(&mut out).map_err(|e| format!("Failed to write ICO bytes: {e}"))?;
    Ok(out)
}

pub async fn fetch_and_save_icon(url_str: &str, id: &str) -> (Option<String>, Option<String>) {
    let parsed_url = match url::Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return (None, None),
    };
    let host = match parsed_url.host_str() {
        Some(h) => h,
        None => return (None, None),
    };

    let favicon_url = format!("https://www.google.com/s2/favicons?domain={}&sz=128", host);
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
    {
        Ok(c) => c,
        Err(_) => return (None, None),
    };

    let resp = match client.get(&favicon_url).send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return (None, None),
    };

    let bytes = match resp.bytes().await {
        Ok(b) if !b.is_empty() => b,
        _ => return (None, None),
    };

    // Ensure profile dir exists
    let profile_dir = get_profile_dir(id);
    let _ = std::fs::create_dir_all(&profile_dir);

    // Save legacy png in icons dir for backward compatibility
    let icons_dir = get_icons_dir();
    let _ = std::fs::create_dir_all(&icons_dir);
    let _ = std::fs::write(icons_dir.join(format!("{}.png", id)), &bytes);

    // Save png in profile dir
    let _ = std::fs::write(profile_dir.join("icon.png"), &bytes);

    // Save platform icon (ICO on Windows, PNG on macOS/Linux)
    let target_icon_path = get_profile_icon_path(id);
    #[cfg(target_os = "windows")]
    let icon_path_str = match convert_to_ico(&bytes) {
        Ok(ico_bytes) => {
            if std::fs::write(&target_icon_path, &ico_bytes).is_ok() {
                Some(target_icon_path.to_string_lossy().to_string())
            } else {
                None
            }
        }
        Err(e) => {
            eprintln!("Warning: could not convert favicon to ICO for container '{id}': {e}");
            None
        }
    };

    #[cfg(not(target_os = "windows"))]
    let icon_path_str = if target_icon_path.exists() || std::fs::write(&target_icon_path, &bytes).is_ok() {
        Some(target_icon_path.to_string_lossy().to_string())
    } else {
        None
    };

    let b64 = format!("data:image/png;base64,{}", BASE64_STANDARD.encode(&bytes));
    (Some(b64), icon_path_str)
}

#[allow(dead_code)]
pub async fn fetch_favicon(url_str: &str, id: &str) -> Option<String> {
    let (icon_b64, _) = fetch_and_save_icon(url_str, id).await;
    icon_b64
}

pub async fn ensure_container_icon(container: &mut ContainerConfig) -> Option<PathBuf> {
    let profile_dir = get_profile_dir(&container.id);
    let _ = std::fs::create_dir_all(&profile_dir);
    let target_icon_file = get_profile_icon_path(&container.id);

    // 1. If target icon already exists and is non-empty, use it
    if target_icon_file.exists() && std::fs::metadata(&target_icon_file).map(|m| m.len() > 0).unwrap_or(false) {
        let path_str = target_icon_file.to_string_lossy().to_string();
        if container.icon_path.as_deref() != Some(&path_str) {
            container.icon_path = Some(path_str);
            let mut all = load_containers();
            if let Some(c) = all.iter_mut().find(|c| c.id.eq_ignore_ascii_case(&container.id)) {
                c.icon_path = container.icon_path.clone();
                let _ = save_containers(&all);
            }
        }
        return Some(target_icon_file);
    }

    // 2. Try converting from existing cached png in profile_dir or icons_dir
    let candidate_pngs = [
        profile_dir.join("icon.png"),
        get_icons_dir().join(format!("{}.png", container.id)),
    ];
    for png_path in &candidate_pngs {
        if png_path.exists() {
            if let Ok(bytes) = std::fs::read(png_path) {
                if !bytes.is_empty() {
                    #[cfg(target_os = "windows")]
                    if let Ok(ico_bytes) = convert_to_ico(&bytes) {
                        if std::fs::write(&target_icon_file, &ico_bytes).is_ok() {
                            let path_str = target_icon_file.to_string_lossy().to_string();
                            container.icon_path = Some(path_str);
                            let mut all = load_containers();
                            if let Some(c) = all.iter_mut().find(|c| c.id.eq_ignore_ascii_case(&container.id)) {
                                c.icon_path = container.icon_path.clone();
                                let _ = save_containers(&all);
                            }
                            return Some(target_icon_file);
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        let _ = std::fs::copy(png_path, &target_icon_file);
                        let path_str = target_icon_file.to_string_lossy().to_string();
                        container.icon_path = Some(path_str);
                        return Some(target_icon_file);
                    }
                }
            }
        }
    }

    // 3. Try converting from container.icon base64 string
    if let Some(b64) = &container.icon {
        if let Some(idx) = b64.find("base64,") {
            let data_str = &b64[idx + 7..];
            if let Ok(bytes) = BASE64_STANDARD.decode(data_str.trim()) {
                if !bytes.is_empty() {
                    #[cfg(target_os = "windows")]
                    if let Ok(ico_bytes) = convert_to_ico(&bytes) {
                        if std::fs::write(&target_icon_file, &ico_bytes).is_ok() {
                            let path_str = target_icon_file.to_string_lossy().to_string();
                            container.icon_path = Some(path_str);
                            let mut all = load_containers();
                            if let Some(c) = all.iter_mut().find(|c| c.id.eq_ignore_ascii_case(&container.id)) {
                                c.icon_path = container.icon_path.clone();
                                let _ = save_containers(&all);
                            }
                            return Some(target_icon_file);
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    if std::fs::write(&target_icon_file, &bytes).is_ok() {
                        let path_str = target_icon_file.to_string_lossy().to_string();
                        container.icon_path = Some(path_str);
                        return Some(target_icon_file);
                    }
                }
            }
        }
    }

    // 4. Try re-fetching from network
    let (icon_b64, icon_path) = fetch_and_save_icon(&container.url, &container.id).await;
    if let Some(p) = icon_path {
        container.icon_path = Some(p.clone());
        if container.icon.is_none() {
            container.icon = icon_b64;
        }
        let mut all = load_containers();
        if let Some(c) = all.iter_mut().find(|c| c.id.eq_ignore_ascii_case(&container.id)) {
            c.icon_path = container.icon_path.clone();
            c.icon = container.icon.clone();
            let _ = save_containers(&all);
        }
        return Some(PathBuf::from(p));
    }

    None
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

    #[test]
    fn test_convert_to_ico_valid() {
        use std::io::Cursor;
        let mut img = image::RgbaImage::new(32, 32);
        for (_, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([255, 0, 0, 255]);
        }
        let mut png_bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png).unwrap();

        let ico_bytes = convert_to_ico(&png_bytes).expect("ICO conversion should succeed");
        assert!(ico_bytes.len() >= 4);
        assert_eq!(&ico_bytes[0..4], &[0, 0, 1, 0]);
    }

    #[test]
    fn test_convert_to_ico_passthrough() {
        let fake_ico = vec![0, 0, 1, 0, 1, 2, 3, 4];
        let res = convert_to_ico(&fake_ico).expect("ICO passthrough should succeed");
        assert_eq!(res, fake_ico);
    }

    #[test]
    fn test_convert_to_ico_invalid_bytes() {
        let corrupt = b"not an image";
        let res = convert_to_ico(corrupt);
        assert!(res.is_err());
    }

    #[test]
    fn test_profile_icon_path() {
        let path = get_profile_icon_path("test-app");
        #[cfg(target_os = "windows")]
        assert!(path.ends_with(PathBuf::from("profiles").join("test-app").join("icon.ico")));
        #[cfg(not(target_os = "windows"))]
        assert!(path.ends_with(PathBuf::from("profiles").join("test-app").join("icon.png")));
    }
}
