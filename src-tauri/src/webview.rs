use tauri::{
    webview::WebviewBuilder, LogicalPosition, LogicalSize, Manager, PhysicalPosition,
    PhysicalSize, Rect, WebviewUrl,
};
use std::thread;
use std::time::Duration;
use std::fs;
use crate::state::{AppState, sanitize_segment, parse_external_url, WEBVIEW_DESTROY_TIMEOUT_SECONDS};
use crate::{web_scripts, security};

pub fn apply_hibernate_script(handle: &tauri::AppHandle, label: &str) {
    if let Some(wv) = handle.get_webview(label) {
        let _ = wv.eval(web_scripts::hibernate_script());
    }
}

pub fn wake_webview(handle: &tauri::AppHandle, label: &str) {
    if let Some(wv) = handle.get_webview(label) {
        let _ = wv.show();
        let _ = wv.eval(web_scripts::wake_script());
    }
}

pub fn destroy_inactive_webview(handle: &tauri::AppHandle, label: &str) {
    println!("  destroy_inactive_webview(label: {})", label);
    if let Some(wv) = handle.get_webview(label) {
        println!("    Found webview, calling close()");
        let _ = wv.close();
    } else {
        println!("    Webview NOT found by label");
    }

    let state = handle.state::<AppState>();
    {
        let mut profiles = state
            .active_profiles
            .lock()
            .expect("active_profiles lock poisoned");
        profiles.remove(label);
    }
    {
        let mut sleep = state
            .app_sleep_seconds
            .lock()
            .expect("app_sleep_seconds lock poisoned");
        sleep.remove(label);
    }
    {
        let mut tokens = state
            .hibernate_tokens
            .lock()
            .expect("hibernate_tokens lock poisoned");
        tokens.remove(label);
    }
}

pub fn schedule_hibernation(handle: &tauri::AppHandle, label: String) {
    if label == "main" {
        return;
    }

    let state = handle.state::<AppState>();
    let delay_seconds = {
        let map = state
            .app_sleep_seconds
            .lock()
            .expect("app_sleep_seconds lock poisoned");
        map.get(&label).copied().unwrap_or(0)
    };

    let token = {
        let mut tokens = state
            .hibernate_tokens
            .lock()
            .expect("hibernate_tokens lock poisoned");
        let entry = tokens.entry(label.clone()).or_insert(0);
        *entry += 1;
        *entry
    };

    if delay_seconds == 0 {
        apply_hibernate_script(handle, &label);
    } else {
        let app_handle = handle.clone();
        let hibernate_label = label.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(delay_seconds));

            let state = app_handle.state::<AppState>();
            let still_inactive = {
                let active = state
                    .active_webview
                    .lock()
                    .expect("active_webview lock poisoned");
                active.as_deref() != Some(hibernate_label.as_str())
            };

            let token_matches = {
                let tokens = state
                    .hibernate_tokens
                    .lock()
                    .expect("hibernate_tokens lock poisoned");
                tokens.get(&hibernate_label).copied().unwrap_or(0) == token
            };

            if still_inactive && token_matches {
                apply_hibernate_script(&app_handle, &hibernate_label);
            }
        });
    }

    let app_handle = handle.clone();
    let destroy_label = label.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(WEBVIEW_DESTROY_TIMEOUT_SECONDS));

        let state = app_handle.state::<AppState>();
        let still_inactive = {
            let active = state
                .active_webview
                .lock()
                .expect("active_webview lock poisoned");
            active.as_deref() != Some(destroy_label.as_str())
        };

        let token_matches = {
            let tokens = state
                .hibernate_tokens
                .lock()
                .expect("hibernate_tokens lock poisoned");
            tokens.get(&destroy_label).copied().unwrap_or(0) == token
        };

        if still_inactive && token_matches {
            destroy_inactive_webview(&app_handle, &destroy_label);
        }
    });
}

pub fn layout_offsets(logical_width: f64) -> (f64, f64, f64) {
    if logical_width < 640.0 {
        (56.0, 56.0, 0.0)
    } else {
        (64.0, 56.0, 24.0)
    }
}

pub fn compute_webview_bounds(
    main_win: &tauri::Window,
) -> Option<(LogicalPosition<f64>, LogicalSize<f64>, Rect)> {
    let scale = main_win.scale_factor().ok().unwrap_or(1.0);
    let size = main_win.inner_size().ok()?;
    let logical_width = (size.width as f64) / scale;
    let logical_height = (size.height as f64) / scale;
    let (sidebar_w, header_h, footer_h) = layout_offsets(logical_width);

    let width = (logical_width - sidebar_w).max(320.0);
    let height = (logical_height - header_h - footer_h).max(220.0);

    let logical_pos = LogicalPosition::new(sidebar_w, header_h);
    let logical_size = LogicalSize::new(width, height);

    let rect = Rect {
        position: PhysicalPosition::new((sidebar_w * scale) as i32, (header_h * scale) as i32).into(),
        size: PhysicalSize::new((width * scale) as u32, (height * scale) as u32).into(),
    };

    Some((logical_pos, logical_size, rect))
}

pub fn set_webview_hibernation(handle: &tauri::AppHandle, active_id: Option<&str>) {
    for (label, wv) in handle.webviews() {
        if label == "main" {
            continue;
        }

        let should_hibernate = match active_id {
            Some(active) => label != active,
            None => true,
        };

        if should_hibernate {
            let _ = wv.hide();
            schedule_hibernation(handle, label);
        } else {
            wake_webview(handle, &label);
        }
    }
}

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

#[allow(clippy::too_many_arguments)]
pub fn orchestrate_app_webview(
    window: tauri::Window,
    handle: tauri::AppHandle,
    state: &AppState,
    id: String,
    url: String,
    app_id: Option<String>,
    profile: Option<String>,
    css: Option<String>,
    custom_css: Option<String>,
    js: Option<String>,
    custom_js: Option<String>,
    injection_allowlist: Option<Vec<String>>,
    idle_sleep_seconds: Option<u64>,
) -> Result<(), String> {
    println!("orchestrate_app_webview(id: {}, app_id: {:?})", id, app_id);
    let root_app_id = app_id.clone().unwrap_or_else(|| {
        println!("  WARNING: app_id was None, using id as root_app_id");
        id.clone()
    });
    let app_key = sanitize_segment(&root_app_id, "app");
    let profile_name = sanitize_segment(profile.as_deref().unwrap_or("default"), "default");
    
    println!("  Resolved app_key: {}, profile: {}", app_key, profile_name);

    let notification_script = web_scripts::notification_hijack_script(&root_app_id);
    let tab_script = web_scripts::new_tab_bridge_script(&root_app_id);
    let external_url = parse_external_url(&url)?;
    let allowlist = security::normalize_allowlist(injection_allowlist);
    let allow_custom_injection = security::is_host_allowed(&external_url, &allowlist);
    let safe_mode_enabled = {
        let safe_mode = state.safe_mode.lock().expect("safe_mode lock poisoned");
        *safe_mode
    };

    let expected_profile = format!("{app_key}:{profile_name}");
    let should_recreate = {
        let profiles = state.active_profiles.lock().expect("active_profiles lock poisoned");
        let current = profiles.get(&id);
        let mismatch = current.map(|saved| saved != &expected_profile).unwrap_or(false);
        if mismatch {
            println!("  Profile mismatch detected: current={:?}, expected={}", current, expected_profile);
        }
        mismatch
    };

    if should_recreate {
        if let Some(existing_wv) = handle.get_webview(&id) {
            println!("  Closing existing webview for recreation");
            let _ = existing_wv.close();
        }
    }

    {
        let mut sleep_seconds = state
            .app_sleep_seconds
            .lock()
            .expect("app_sleep_seconds lock poisoned");
        sleep_seconds.insert(id.clone(), idle_sleep_seconds.unwrap_or(0).min(600));
    }

    {
        let mut active = state
            .active_webview
            .lock()
            .expect("active_webview lock poisoned");
        *active = Some(id.clone());
    }

    {
        let mut tokens = state
            .hibernate_tokens
            .lock()
            .expect("hibernate_tokens lock poisoned");
        let entry = tokens.entry(id.clone()).or_insert(0);
        *entry += 1;
    }

    set_webview_hibernation(&handle, Some(&id));

    if let Some(existing_wv) = handle.get_webview(&id) {
        println!("  Found existing webview, updating bounds and scripts");
        if let Some((_, _, rect)) = compute_webview_bounds(&window) {
            let _ = existing_wv.set_bounds(rect);
        }
        let _ = existing_wv.show();
        let _ = existing_wv.set_focus();
        let _ = existing_wv.eval(web_scripts::wake_script());

        let _ = existing_wv.eval(&notification_script);
        let _ = existing_wv.eval(&tab_script);

        if let Some(css_code) = css {
            if !css_code.trim().is_empty() && security::validate_css_payload(&css_code).is_ok() {
                let _ = existing_wv.eval(&web_scripts::upsert_style_script("vessel-base-css", &css_code));
            }
        }

        if allow_custom_injection {
            if let Some(custom_css_code) = custom_css {
                if !custom_css_code.trim().is_empty() && security::validate_css_payload(&custom_css_code).is_ok() {
                    let _ = existing_wv.eval(&web_scripts::upsert_style_script("vessel-custom-css", &custom_css_code));
                }
            }
        }

        if !safe_mode_enabled {
            if let Some(js_code) = js {
                if !js_code.trim().is_empty() && security::validate_js_payload(&js_code).is_ok() {
                    let _ = existing_wv.eval(&web_scripts::upsert_runtime_js_script("vessel-runtime-js", &js_code));
                }
            }

            if let Some(custom_js_code) = custom_js {
                if !custom_js_code.trim().is_empty() && allow_custom_injection && security::validate_js_payload(&custom_js_code).is_ok() {
                    let _ = existing_wv.eval(&web_scripts::upsert_runtime_js_script("vessel-custom-js", &custom_js_code));
                }
            }
        }

        let mut profiles = state.active_profiles.lock().expect("active_profiles lock poisoned");
        profiles.insert(id, expected_profile);
        return Ok(());
    }

    println!("  Creating new webview child");
    let Some((logical_pos, logical_size, _)) = compute_webview_bounds(&window) else {
        return Err("unable to compute webview bounds".to_string());
    };

    let mut builder = WebviewBuilder::new(&id, WebviewUrl::External(external_url))
        .user_agent(get_stealth_user_agent());

    let session_dir = state
        .sessions_root
        .join(&app_key)
        .join(&profile_name);
    let _ = fs::create_dir_all(&session_dir);
    println!("  Session directory: {:?}", session_dir);
    builder = builder.data_directory(session_dir);

    builder = builder.initialization_script(web_scripts::stealth_injection_script());
    builder = builder.initialization_script(&notification_script);
    builder = builder.initialization_script(&tab_script);

    if let Some(css_code) = css {
        if !css_code.trim().is_empty() && security::validate_css_payload(&css_code).is_ok() {
            builder = builder.initialization_script(&web_scripts::upsert_style_script("vessel-base-css", &css_code));
        }
    }

    if allow_custom_injection {
        if let Some(custom_css_code) = custom_css {
            if !custom_css_code.trim().is_empty() && security::validate_css_payload(&custom_css_code).is_ok() {
                builder = builder.initialization_script(&web_scripts::upsert_style_script("vessel-custom-css", &custom_css_code));
            }
        }
    }

    if !safe_mode_enabled {
        if let Some(js_code) = js {
            if !js_code.trim().is_empty() && security::validate_js_payload(&js_code).is_ok() {
                builder = builder.initialization_script(&web_scripts::upsert_runtime_js_script("vessel-runtime-js", &js_code));
            }
        }
        if let Some(custom_js_code) = custom_js {
            if !custom_js_code.trim().is_empty() && allow_custom_injection && security::validate_js_payload(&custom_js_code).is_ok() {
                builder = builder.initialization_script(&web_scripts::upsert_runtime_js_script("vessel-custom-js", &custom_js_code));
            }
        }
    }

    window
        .add_child(
            builder,
            logical_pos,
            logical_size,
        )
        .map_err(|e| format!("failed to create webview: {e}"))?;

    let mut profiles = state.active_profiles.lock().expect("active_profiles lock poisoned");
    profiles.insert(id, expected_profile);
    Ok(())
}
