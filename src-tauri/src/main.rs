#![windows_subsystem = "windows"]

mod profile;
mod commands;
pub mod supervisor;
pub mod vault;

use std::sync::{Arc, Mutex};
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

pub fn get_root_domain(host: &str) -> &str {
    let host = host.trim().trim_start_matches("www.");
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() <= 2 {
        return host;
    }
    let last = parts[parts.len() - 1];
    let second_last = parts[parts.len() - 2];
    if ["co", "com", "org", "net", "gov", "edu"].contains(&second_last) && parts.len() >= 3 {
        let third_last = parts[parts.len() - 3];
        let offset = host.len() - (third_last.len() + second_last.len() + last.len() + 2);
        return &host[offset..];
    }
    let offset = host.len() - (second_last.len() + last.len() + 1);
    &host[offset..]
}

pub fn is_allowed_container_navigation(url: &url::Url, base_host: &str, root_domain: &str) -> bool {
    let scheme = url.scheme();
    // Allow internal browser schemes
    if scheme == "about" || scheme == "blob" || scheme == "data" || scheme == "javascript" {
        return true;
    }

    let target_host = match url.host_str() {
        Some(h) => h.to_lowercase(),
        None => return true,
    };

    // 1. Same host or subdomain (e.g. app.linear.app vs linear.app)
    if target_host == base_host || target_host.ends_with(&format!(".{base_host}")) {
        return true;
    }

    // 2. Same root domain (e.g. accounts.spotify.com vs open.spotify.com)
    if !root_domain.is_empty() && (target_host == root_domain || target_host.ends_with(&format!(".{root_domain}"))) {
        return true;
    }

    // 3. Known OAuth / SSO / Identity Providers
    const KNOWN_AUTH_DOMAINS: &[&str] = &[
        "accounts.google.com",
        "login.microsoftonline.com",
        "login.live.com",
        "appleid.apple.com",
        "github.com",
        "gitlab.com",
        "bitbucket.org",
        "auth0.com",
        "okta.com",
        "onelogin.com",
        "clerk.com",
        "clerk.accounts.dev",
        "identity.atlassian.com",
        "auth.atlassian.com",
        "sso.slack.com",
        "discord.com",
        "facebook.com",
        "twitter.com",
        "x.com",
        "linkedin.com",
    ];

    if KNOWN_AUTH_DOMAINS.iter().any(|&auth| target_host == auth || target_host.ends_with(&format!(".{auth}"))) {
        return true;
    }

    false
}

pub fn open_or_focus_container(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let win_label = format!("container-{}", id);
    if let Some(win) = app.get_webview_window(&win_label) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        if let Some(state) = app.try_state::<supervisor::SupervisorState>() {
            if let Ok(mut sup) = state.lock() {
                let pid = supervisor::extract_window_pid(&win);
                let title = win.title().unwrap_or_default();
                let url_str = win.url().map(|u| u.to_string()).unwrap_or_default();
                sup.register_or_activate(id, &title, &url_str, pid);
            }
        }
        return Ok(());
    }

    let container = profile::get_container(id)
        .ok_or_else(|| format!("Container '{id}' is not registered in profiles.json"))?;

    // Check if an encrypted vault exists. If so, decrypt and unpack before launching webview
    let vault_path = vault::get_vault_path(id);
    if vault_path.exists() {
        let key = vault::get_or_create_vault_key(id)
            .map_err(|e| format!("Failed to retrieve vault key from Keyring for '{id}': {e}"))?;
        let profile_dest = profile::get_profile_dir(id);
        vault::decrypt_and_unpack_vault(id, &vault_path, &profile_dest, &key)
            .map_err(|e| format!("Failed to unlock encrypted vault for '{id}': {e}"))?;
    }

    let target_url_str = if let Some(state) = app.try_state::<supervisor::SupervisorState>() {
        let lock = state.lock().ok();
        lock.and_then(|s| s.get_container(id))
            .filter(|c| !c.current_url.is_empty())
            .map(|c| c.current_url)
            .unwrap_or_else(|| container.url.clone())
    } else {
        container.url.clone()
    };

    let external_url = url::Url::parse(&target_url_str)
        .or_else(|_| url::Url::parse(&container.url))
        .map_err(|e| format!("Invalid container URL '{}': {e}", target_url_str))?;

    let profile_dir = profile::get_profile_dir(&container.id);
    let _ = std::fs::create_dir_all(&profile_dir);

    let initial_url = external_url.clone();
    let base_host = external_url.host_str().unwrap_or("").to_lowercase();
    let root_domain = get_root_domain(&base_host).to_string();

    let app_handle_nav = app.clone();
    let id_nav = id.to_string();

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
    .data_directory(profile_dir)
    .on_navigation(move |nav_url| {
        if let Some(state) = app_handle_nav.try_state::<supervisor::SupervisorState>() {
            if let Ok(mut sup) = state.lock() {
                sup.update_url(&id_nav, nav_url.as_str());
            }
        }

        // Ensure the initial load of container.url always passes the check
        if nav_url == &initial_url || nav_url.as_str() == initial_url.as_str() {
            return true;
        }

        if is_allowed_container_navigation(nav_url, &base_host, &root_domain) {
            true
        } else {
            // Open external URLs in default system browser
            let _ = tauri_plugin_opener::open_url(nav_url.as_str(), None::<&str>);
            false
        }
    });

    let win = builder.build().map_err(|e| format!("Failed to create container window for '{id}': {e}"))?;

    let win_pid = supervisor::extract_window_pid(&win);
    if let Some(state) = app.try_state::<supervisor::SupervisorState>() {
        if let Ok(mut sup) = state.lock() {
            sup.register_or_activate(id, &container.name, &target_url_str, win_pid);
        }
    }

    let app_handle_events = app.clone();
    let id_events = id.to_string();
    win.on_window_event(move |event| {
        if let Some(state) = app_handle_events.try_state::<supervisor::SupervisorState>() {
            match event {
                tauri::WindowEvent::Focused(focused) => {
                    if let Ok(mut sup) = state.lock() {
                        sup.update_focus(&id_events, *focused);
                    }
                }
                tauri::WindowEvent::Destroyed => {
                    if let Ok(mut sup) = state.lock() {
                        sup.mark_closed(&id_events);
                    }

                    // Automatic vault sealing on container close
                    let id_vault = id_events.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        let profile_dir = profile::get_profile_dir(&id_vault);
                        if profile_dir.exists() {
                            let vault_dest = vault::get_vault_path(&id_vault);
                            if let Ok(key) = vault::get_or_create_vault_key(&id_vault) {
                                if let Err(e) = vault::pack_and_encrypt_vault(&id_vault, &profile_dir, &vault_dest, &key) {
                                    eprintln!("[Vault] Automatic lock on close failed for '{id_vault}': {e}");
                                } else {
                                    println!("[Vault] Container '{id_vault}' automatically locked and encrypted at rest");
                                }
                            }
                        }
                    });
                }
                _ => {}
            }
        }
    });

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

    let mut init_supervisor = supervisor::ProcessSupervisor::new(supervisor::SupervisorConfig::default());
    for container in profile::load_containers() {
        init_supervisor.register_container(&container.id, &container.name, &container.url);
    }
    let supervisor: supervisor::SupervisorState = Arc::new(Mutex::new(init_supervisor));

    tauri::Builder::default()
        .manage(supervisor.clone())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(target_id) = parse_run_container_id(&argv) {
                let _ = open_or_focus_container(app, &target_id);
            } else {
                let _ = open_manager_window(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            supervisor::start_idle_monitor(app.handle().clone(), supervisor.clone());

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
            commands::create_shortcut,
            commands::get_container_metrics,
            commands::wake_container,
            commands::force_hibernate_container,
            commands::lock_container_vault,
            commands::unlock_container_vault,
            commands::is_vault_locked
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vessel");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_domain_extraction() {
        assert_eq!(get_root_domain("linear.app"), "linear.app");
        assert_eq!(get_root_domain("app.linear.app"), "linear.app");
        assert_eq!(get_root_domain("open.spotify.com"), "spotify.com");
        assert_eq!(get_root_domain("sub.domain.example.co.uk"), "example.co.uk");
        assert_eq!(get_root_domain("www.github.com"), "github.com");
    }

    #[test]
    fn test_allowed_container_navigation() {
        let base_host = "linear.app";
        let root_domain = "linear.app";

        // Same domain
        let url_same = url::Url::parse("https://linear.app/my-team/issue").unwrap();
        assert!(is_allowed_container_navigation(&url_same, base_host, root_domain));

        // Subdomain
        let url_sub = url::Url::parse("https://app.linear.app/settings").unwrap();
        assert!(is_allowed_container_navigation(&url_sub, base_host, root_domain));

        // Browser internal scheme
        let url_about = url::Url::parse("about:blank").unwrap();
        assert!(is_allowed_container_navigation(&url_about, base_host, root_domain));

        // Known OAuth identity providers
        let url_google = url::Url::parse("https://accounts.google.com/o/oauth2/auth?client_id=123").unwrap();
        assert!(is_allowed_container_navigation(&url_google, base_host, root_domain));

        let url_ms = url::Url::parse("https://login.microsoftonline.com/common/oauth2/authorize").unwrap();
        assert!(is_allowed_container_navigation(&url_ms, base_host, root_domain));

        let url_apple = url::Url::parse("https://appleid.apple.com/auth/authorize").unwrap();
        assert!(is_allowed_container_navigation(&url_apple, base_host, root_domain));

        let url_github = url::Url::parse("https://github.com/login/oauth/authorize").unwrap();
        assert!(is_allowed_container_navigation(&url_github, base_host, root_domain));

        let url_okta = url::Url::parse("https://company.okta.com/oauth2/v1/authorize").unwrap();
        assert!(is_allowed_container_navigation(&url_okta, base_host, root_domain));

        let url_auth0 = url::Url::parse("https://company.auth0.com/authorize").unwrap();
        assert!(is_allowed_container_navigation(&url_auth0, base_host, root_domain));

        let url_clerk = url::Url::parse("https://subdomain.clerk.accounts.dev/sign-in").unwrap();
        assert!(is_allowed_container_navigation(&url_clerk, base_host, root_domain));

        // External third-party URLs (must be blocked in container so they open in external browser)
        let url_news = url::Url::parse("https://news.ycombinator.com/item?id=456").unwrap();
        assert!(!is_allowed_container_navigation(&url_news, base_host, root_domain));

        let url_external = url::Url::parse("https://www.theverge.com/tech").unwrap();
        assert!(!is_allowed_container_navigation(&url_external, base_host, root_domain));
    }

    #[test]
    fn test_create_shortcut_existing_container() {
        if profile::get_container("spotify").is_some() {
            let res = tauri::async_runtime::block_on(commands::create_shortcut("spotify".to_string()));
            assert!(res.is_ok(), "create_shortcut for spotify should succeed: {:?}", res);

            let ico_path = profile::get_profile_icon_path("spotify");
            assert!(ico_path.exists(), "Profile icon file should exist at {:?}", ico_path);

            let meta = std::fs::metadata(&ico_path).expect("Icon metadata should be readable");
            assert!(meta.len() > 0, "Icon file should not be empty");

            // Verify container config was updated with icon_path
            let updated = profile::get_container("spotify").expect("Container should exist");
            assert!(updated.icon_path.is_some(), "Container icon_path should be populated");
        }

        if profile::get_container("github").is_some() {
            let res = tauri::async_runtime::block_on(commands::create_shortcut("github".to_string()));
            assert!(res.is_ok(), "create_shortcut for github should succeed: {:?}", res);

            let ico_path = profile::get_profile_icon_path("github");
            assert!(ico_path.exists(), "Profile icon file should exist at {:?}", ico_path);

            let updated = profile::get_container("github").expect("Container should exist");
            assert!(updated.icon_path.is_some(), "Container icon_path should be populated");
        }
    }
}

