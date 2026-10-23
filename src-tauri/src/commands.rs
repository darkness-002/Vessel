use tauri::Manager;
use crate::profile::{self, ContainerConfig};

#[tauri::command]
pub fn list_containers() -> Result<Vec<ContainerConfig>, String> {
    Ok(profile::load_containers())
}

#[tauri::command]
pub async fn create_container(app: tauri::AppHandle, name: String, url: String) -> Result<ContainerConfig, String> {
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
    let (icon, icon_path) = profile::fetch_and_save_icon(&clean_url, &id).await;

    let config = ContainerConfig {
        id: id.clone(),
        name: clean_name,
        url: clean_url,
        icon,
        icon_path,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    containers.push(config.clone());
    profile::save_containers(&containers)?;

    if let Some(state) = app.try_state::<crate::supervisor::SupervisorState>() {
        if let Ok(mut sup) = state.lock() {
            sup.register_container(&config.id, &config.name, &config.url);
        }
    }

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

    // 3. Remove from supervisor
    if let Some(state) = app.try_state::<crate::supervisor::SupervisorState>() {
        if let Ok(mut sup) = state.lock() {
            sup.remove_container(&id);
        }
    }

    // 4. Delete isolated profile data directory
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

    // 5. Remove icon if present
    let icon_file = profile::get_icons_dir().join(format!("{}.png", id));
    if icon_file.exists() {
        let _ = std::fs::remove_file(icon_file);
    }
    let profile_icon = profile::get_profile_icon_path(&id);
    if profile_icon.exists() {
        let _ = std::fs::remove_file(profile_icon);
    }

    // 6. Remove vault file and delete credentials in keyring
    let vault_file = crate::vault::get_vault_path(&id);
    if vault_file.exists() {
        let _ = std::fs::remove_file(vault_file);
    }
    let _ = crate::vault::delete_vault_key(&id);

    Ok(())
}

#[tauri::command]
pub async fn launch_container(app: tauri::AppHandle, id: String) -> Result<(), String> {
    crate::open_or_focus_container(&app, &id)
}

#[tauri::command]
pub fn get_container_metrics(
    supervisor: tauri::State<'_, crate::supervisor::SupervisorState>,
) -> Result<Vec<crate::supervisor::ContainerProcessMetrics>, String> {
    let mut lock = supervisor.lock().map_err(|e| e.to_string())?;
    Ok(lock.get_metrics())
}

#[tauri::command]
pub async fn wake_container(app: tauri::AppHandle, id: String) -> Result<(), String> {
    crate::open_or_focus_container(&app, &id)
}

#[tauri::command]
pub async fn force_hibernate_container(
    app: tauri::AppHandle,
    supervisor: tauri::State<'_, crate::supervisor::SupervisorState>,
    id: String,
) -> Result<String, String> {
    crate::supervisor::hibernate_container(&app, &supervisor, &id)
}

#[tauri::command]
pub async fn lock_container_vault(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let win_label = format!("container-{}", id);
    if let Some(win) = app.get_webview_window(&win_label) {
        let _ = win.destroy();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    if let Some(state) = app.try_state::<crate::supervisor::SupervisorState>() {
        if let Ok(mut sup) = state.lock() {
            sup.mark_closed(&id);
        }
    }

    let profile_dir = profile::get_profile_dir(&id);
    let vault_dest = crate::vault::get_vault_path(&id);

    let key = crate::vault::get_or_create_vault_key(&id)
        .map_err(|e| format!("Keyring error: {e}"))?;

    crate::vault::pack_and_encrypt_vault(&id, &profile_dir, &vault_dest, &key)
        .map_err(|e| format!("Vault encryption failed: {e}"))?;

    Ok(format!("Vault for container '{id}' locked and encrypted at rest"))
}

#[tauri::command]
pub async fn unlock_container_vault(
    id: String,
    passphrase: Option<String>,
) -> Result<String, String> {
    let vault_path = crate::vault::get_vault_path(&id);
    if !vault_path.exists() {
        return Ok(format!("Container '{id}' is not locked in a vault"));
    }

    let profile_dest = profile::get_profile_dir(&id);

    let key = if let Some(pass) = passphrase.filter(|p| !p.trim().is_empty()) {
        let salt = crate::vault::read_vault_salt(&vault_path)
            .map_err(|e| format!("Failed to read vault header: {e}"))?;
        crate::vault::derive_key_from_passphrase(&pass, &salt)
            .map_err(|e| format!("Passphrase key derivation failed: {e}"))?
    } else {
        crate::vault::get_or_create_vault_key(&id)
            .map_err(|e| format!("Keyring retrieval failed: {e}"))?
    };

    crate::vault::decrypt_and_unpack_vault(&id, &vault_path, &profile_dest, &key)
        .map_err(|e| format!("Decryption failed: {e}"))?;

    Ok(format!("Vault for container '{id}' unlocked successfully"))
}

#[tauri::command]
pub fn is_vault_locked(id: String) -> bool {
    crate::vault::is_vault_locked(&id)
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
    let mut container = profile::get_container(&id)
        .ok_or_else(|| format!("Container '{}' not found", id))?;

    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {e}"))?;

    // Ensure icon exists on disk (converts existing PNG / Base64 / online if needed)
    let icon_file = profile::ensure_container_icon(&mut container).await;

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

        let icon_str = if let Some(ico_path) = icon_file {
            format!("{},0", ico_path.to_string_lossy())
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

        if let Some(icon_path) = icon_file {
            let icon_str = icon_path.to_string_lossy().to_string();
            let shortcut_str = shortcut_path.to_string_lossy().to_string();
            let osascript_cmd = format!(
                "use framework \"Cocoa\"\n\
                 set iconImage to (current application's NSImage's alloc()'s initWithContentsOfFile:\"{icon}\")\n\
                 if iconImage is not missing value then\n\
                     (current application's NSWorkspace's sharedWorkspace()'s setIcon:iconImage forFile:\"{shortcut}\" options:0)\n\
                 end if",
                icon = icon_str.replace('"', "\\\""),
                shortcut = shortcut_str.replace('"', "\\\""),
            );
            let _ = std::process::Command::new("osascript")
                .args(["-e", &osascript_cmd])
                .output();
        }

        Ok(format!("Desktop shortcut created for \"{}\"", container.name))
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let desktop_dir = resolve_desktop_dir()?;
        let safe_name = id.replace(' ', "-");
        let shortcut_path = desktop_dir.join(format!("{}.desktop", safe_name));
        let icon_str = icon_file
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "application-default-icon".to_string());
        let content = format!(
            "[Desktop Entry]\nType=Application\nName={}\nExec=\"{}\" --run-container \"{}\"\nIcon={}\nTerminal=false\n",
            container.name,
            current_exe.display(),
            id,
            icon_str
        );
        std::fs::write(&shortcut_path, content).map_err(|e| format!("Failed to write .desktop file: {e}"))?;
        Ok(format!("Desktop shortcut created for \"{}\"", container.name))
    }
}
