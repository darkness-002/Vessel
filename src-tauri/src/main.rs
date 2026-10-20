#![windows_subsystem = "windows"]

mod profile;
mod commands;

use tauri::{Manager, WebviewUrl, webview::WebviewWindowBuilder};

pub fn get_stealth_user_agent() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.3 Safari/605.1.15"
    }
    #[cfg(target_os = "windows")]
    {
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36 Edg/122.0.0.0"
    }
    #[cfg(target_os = "linux")]
    {
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"
    }
}

pub fn open_manager_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("manager") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        return Ok(());
    }

    let builder = WebviewWindowBuilder::new(
        app,
        "manager",
        WebviewUrl::App("index.html".into()),
    )
    .title("Vessel")
    .inner_size(960.0, 680.0)
    .min_inner_size(700.0, 500.0)
    .resizable(true);

    builder.build().map_err(|e| format!("Failed to create Manager window: {e}"))?;
    Ok(())
}

pub fn open_or_focus_container(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let win_label = format!("container-{}", id);
    if let Some(win) = app.get_webview_window(&win_label) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        return Ok(());
    }

    let container = profile::get_container(id)
        .ok_or_else(|| format!("Container '{id}' is not registered in profiles.json"))?;

    let external_url = url::Url::parse(&container.url)
        .map_err(|e| format!("Invalid container URL '{}': {e}", container.url))?;

    let profile_dir = profile::get_profile_dir(&container.id);
    let _ = std::fs::create_dir_all(&profile_dir);

    let builder = WebviewWindowBuilder::new(
        app,
        &win_label,
        WebviewUrl::External(external_url),
    )
    .title(&container.name)
    .inner_size(1200.0, 800.0)
    .min_inner_size(600.0, 450.0)
    .resizable(true)
    .user_agent(get_stealth_user_agent())
    .data_directory(profile_dir);

    builder.build().map_err(|e| format!("Failed to create container window for '{id}': {e}"))?;
    Ok(())
}

fn parse_run_container_id(args: &[String]) -> Option<String> {
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--run-container" && i + 1 < args.len() {
            return Some(args[i + 1].trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(stripped) = arg.strip_prefix("--run-container=") {
            return Some(stripped.trim_matches('"').trim_matches('\'').to_string());
        }
        i += 1;
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let initial_run_container = parse_run_container_id(&args);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(target_id) = parse_run_container_id(&argv) {
                let _ = open_or_focus_container(app, &target_id);
            } else {
                let _ = open_manager_window(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            if let Some(container_id) = initial_run_container {
                // Container Mode: vessel --run-container <ID>
                // Boots ONLY the targeted WebviewWindow pointing to the container URL.
                // Never load or render the Manager DOM in this mode.
                if let Err(e) = open_or_focus_container(&app.handle(), &container_id) {
                    eprintln!("Error booting container '{container_id}': {e}");
                    // If invalid ID, fallback to Manager
                    let _ = open_manager_window(&app.handle());
                }
            } else {
                // Manager Mode: vessel (no arguments)
                // Boots standard Manager UI to create, list, and launch containers.
                open_manager_window(&app.handle())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_containers,
            commands::create_container,
            commands::delete_container,
            commands::launch_container,
            commands::create_shortcut
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vessel");
}
