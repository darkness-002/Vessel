use tauri::Manager;
use crate::profile::{self, ContainerConfig};

#[tauri::command]
pub fn list_containers() -> Result<Vec<ContainerConfig>, String> {
    Ok(profile::load_containers())
}

#[tauri::command]
pub async fn create_container(name: String, url: String) -> Result<ContainerConfig, String> {
    let clean_name = name.trim().to_string();
    if clean_name.is_empty() {
        return Err("Container name cannot be empty".to_string());
    }
    let clean_url = profile::normalize_url(&url);
    if url::Url::parse(&clean_url).is_err() {
        return Err("Invalid URL format".to_string());
    }

    let mut containers = profile::load_containers();
    let base_id = profile::sanitize_id(&clean_name);
    let mut id = base_id.clone();
    let mut counter = 1;
    while containers.iter().any(|c| c.id.eq_ignore_ascii_case(&id)) {
        id = format!("{}-{}", base_id, counter);
        counter += 1;
    }

    // Generate isolated container profile directory
    let profile_dir = profile::get_profile_dir(&id);
    std::fs::create_dir_all(&profile_dir)
        .map_err(|e| format!("Failed to create container profile directory: {e}"))?;

    // Fetch and save favicon
    let icon = profile::fetch_favicon(&clean_url, &id).await;

    let config = ContainerConfig {
        id: id.clone(),
        name: clean_name,
        url: clean_url,
        icon,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    containers.push(config.clone());
    profile::save_containers(&containers)?;

    Ok(config)
}

#[tauri::command]
pub async fn delete_container(app: tauri::AppHandle, id: String) -> Result<(), String> {
    // 1. Close any running window for this container
    let win_label = format!("container-{}", id);
    if let Some(win) = app.get_webview_window(&win_label) {
        let _ = win.close();
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // 2. Remove config entry from profiles.json
    let mut containers = profile::load_containers();
    containers.retain(|c| !c.id.eq_ignore_ascii_case(&id));
    profile::save_containers(&containers)?;

    // 3. Delete isolated profile data directory
    let profile_dir = profile::get_profile_dir(&id);
    if profile_dir.exists() {
        let mut last_err = String::new();
        for i in 0..5 {
            if i > 0 {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            match std::fs::remove_dir_all(&profile_dir) {
                Ok(_) => { last_err.clear(); break; },
                Err(e) => last_err = e.to_string(),
            }
        }
        if !last_err.is_empty() {
            eprintln!("Warning: could not delete profile dir {profile_dir:?}: {last_err}");
        }
    }

    // 4. Remove icon if present
    let icon_file = profile::get_icons_dir().join(format!("{}.png", id));
    if icon_file.exists() {
        let _ = std::fs::remove_file(icon_file);
    }

    Ok(())
}

#[tauri::command]
pub async fn launch_container(app: tauri::AppHandle, id: String) -> Result<(), String> {
    crate::open_or_focus_container(&app, &id)
}

pub fn resolve_desktop_dir() -> Result<std::path::PathBuf, String> {
    // 1. Try dirs::desktop_dir() which queries SHGetKnownFolderPath(FOLDERID_Desktop) on Windows
    if let Some(dir) = dirs::desktop_dir() {
        if dir.exists() {
            return Ok(dir);
        }
    }

    // 2. Windows fallback: check OneDrive KFM Desktop and standard Desktop
    #[cfg(target_os = "windows")]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let onedrive_desktop = std::path::PathBuf::from(&profile).join("OneDrive").join("Desktop");
            if onedrive_desktop.exists() {
                return Ok(onedrive_desktop);
            }
            let standard_desktop = std::path::PathBuf::from(&profile).join("Desktop");
            if standard_desktop.exists() {
                return Ok(standard_desktop);
            }
        }
    }

    // 3. Unix fallback: check HOME/Desktop
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let desktop = std::path::PathBuf::from(&home).join("Desktop");
            if desktop.exists() {
                return Ok(desktop);
            }
        }
    }

    dirs::desktop_dir().ok_or_else(|| "Could not locate Desktop directory".to_string())
}

#[tauri::command]
pub async fn create_shortcut(id: String) -> Result<String, String> {
    let container = profile::get_container(&id)
        .ok_or_else(|| format!("Container '{}' not found", id))?;

    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {e}"))?;

    #[cfg(target_os = "windows")]
    {
        let desktop_dir = resolve_desktop_dir()?;

        let safe_name: String = container.name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let safe_name = if safe_name.trim().is_empty() { id.clone() } else { safe_name.trim().to_string() };

        let shortcut_path = desktop_dir.join(format!("{}.lnk", safe_name));
        let exe_str = current_exe.to_string_lossy().to_string();
        let work_dir_str = current_exe.parent().unwrap_or(&desktop_dir).to_string_lossy().to_string();
        let shortcut_str = shortcut_path.to_string_lossy().to_string();
        let args_str = format!("--run-container \"{}\"", id);

        let icon_file = profile::get_icons_dir().join(format!("{}.png", id));
        let icon_str = if icon_file.exists() {
            icon_file.to_string_lossy().to_string()
        } else {
            format!("{exe_str},0")
        };

        let ps_script = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $s = $ws.CreateShortcut('{shortcut}'); \
             $s.TargetPath = '{exe}'; \
             $s.Arguments = '{args}'; \
             $s.WorkingDirectory = '{workdir}'; \
             $s.Description = '{name} in Vessel'; \
             $s.IconLocation = '{icon}'; \
             $s.Save()",
            shortcut = shortcut_str.replace('\'', "''"),
            exe = exe_str.replace('\'', "''"),
            args = args_str.replace('\'', "''"),
            workdir = work_dir_str.replace('\'', "''"),
            name = container.name.replace('\'', "''"),
            icon = icon_str.replace('\'', "''"),
        );

        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_script]);

        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output().map_err(|e| format!("PowerShell error: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Shortcut creation failed: {stderr}"));
        }

        Ok(format!("Desktop shortcut created for \"{}\"", container.name))
    }

    #[cfg(target_os = "macos")]
    {
        let desktop_dir = resolve_desktop_dir()?;
        let safe_name: String = container.name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let shortcut_path = desktop_dir.join(format!("{}.command", safe_name.trim()));
        let script = format!("#!/bin/sh\n\"{}\" --run-container \"{}\" &\n", current_exe.display(), id);
        std::fs::write(&shortcut_path, script).map_err(|e| format!("Failed to write script: {e}"))?;
        let _ = std::process::Command::new("chmod").args(["+x", &shortcut_path.to_string_lossy()]).output();
        Ok(format!("Desktop shortcut created for \"{}\"", container.name))
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let desktop_dir = resolve_desktop_dir()?;
        let safe_name = id.replace(' ', "-");
        let shortcut_path = desktop_dir.join(format!("{}.desktop", safe_name));
        let content = format!(
            "[Desktop Entry]\nType=Application\nName={}\nExec=\"{}\" --run-container \"{}\"\nIcon=application-default-icon\nTerminal=false\n",
            container.name,
            current_exe.display(),
            id
        );
        std::fs::write(&shortcut_path, content).map_err(|e| format!("Failed to write .desktop file: {e}"))?;
        Ok(format!("Desktop shortcut created for \"{}\"", container.name))
    }
}
