#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::Duration,
};

use chrono::Local;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use url::Url;
use tauri::{
    webview::WebviewBuilder, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalPosition,
    PhysicalSize, Rect, State, WebviewUrl, WindowEvent,
};

mod metrics;
mod security;
mod web_scripts;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VesselNotification {
    app_id: String,
    title: String,
    body: String,
    time: String,
}

struct AppState {
    db_path: PathBuf,
    sessions_root: PathBuf,
    active_profiles: Mutex<HashMap<String, String>>,
    app_sleep_seconds: Mutex<HashMap<String, u64>>,
    active_webview: Mutex<Option<String>>,
    hibernate_tokens: Mutex<HashMap<String, u64>>,
    safe_mode: Mutex<bool>,
}

const WEBVIEW_DESTROY_TIMEOUT_SECONDS: u64 = 600;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceUsage {
    cpu_percent: f32,
    ram_mb: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NewTabRequest {
    app_id: String,
    url: String,
    title: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DiagnosticEvent {
    level: String,
    category: String,
    app_id: String,
    webview_id: Option<String>,
    message: String,
    detail: Option<String>,
    time: String,
}

fn emit_diagnostic(
    app: &tauri::AppHandle,
    level: &str,
    category: &str,
    app_id: &str,
    webview_id: Option<&str>,
    message: &str,
    detail: Option<String>,
) {
    let event = DiagnosticEvent {
        level: level.to_string(),
        category: category.to_string(),
        app_id: app_id.to_string(),
        webview_id: webview_id.map(|value| value.to_string()),
        message: message.to_string(),
        detail,
        time: Local::now().format("%I:%M %p").to_string(),
    };

    let _ = app.emit("vessel-diagnostic", event);
}

fn get_stealth_user_agent() -> &'static str {
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

fn sanitize_segment(input: &str, fallback: &str) -> String {
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

fn notification_db_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("notifications.sqlite")
}

fn sessions_root_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("sessions")
}

fn ensure_db(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create db directory: {e}"))?;
    }

    let conn = Connection::open(path).map_err(|e| format!("failed to open notification db: {e}"))?;
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS notifications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            app_id TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            time TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_notifications_created_at
        ON notifications(created_at DESC);
        ",
    )
    .map_err(|e| format!("failed to initialize notification db schema: {e}"))?;

    Ok(())
}

fn insert_notification(path: &Path, note: &VesselNotification) -> Result<(), String> {
    let conn = Connection::open(path).map_err(|e| format!("failed to open notification db: {e}"))?;
    conn.execute(
        "
        INSERT INTO notifications (app_id, title, body, time, created_at)
        VALUES (?1, ?2, ?3, ?4, strftime('%s','now'))
        ",
        params![note.app_id, note.title, note.body, note.time],
    )
    .map_err(|e| format!("failed to insert notification: {e}"))?;

    // Keep history bounded for resource efficiency.
    conn.execute(
        "
        DELETE FROM notifications
        WHERE id NOT IN (
            SELECT id FROM notifications
            ORDER BY created_at DESC, id DESC
            LIMIT 500
        )
        ",
        [],
    )
    .map_err(|e| format!("failed to trim notification history: {e}"))?;

    Ok(())
}

fn list_notifications(path: &Path, limit: u32) -> Result<Vec<VesselNotification>, String> {
    let conn = Connection::open(path).map_err(|e| format!("failed to open notification db: {e}"))?;
    let mut stmt = conn
        .prepare(
            "
            SELECT app_id, title, body, time
            FROM notifications
            ORDER BY created_at DESC, id DESC
            LIMIT ?1
            ",
        )
        .map_err(|e| format!("failed to query notifications: {e}"))?;

    let mut rows = stmt
        .query(params![limit])
        .map_err(|e| format!("failed to read notifications: {e}"))?;

    let mut result = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("failed to iterate notifications: {e}"))?
    {
        result.push(VesselNotification {
            app_id: row.get::<_, String>(0).unwrap_or_default(),
            title: row.get::<_, String>(1).unwrap_or_default(),
            body: row.get::<_, String>(2).unwrap_or_default(),
            time: row.get::<_, String>(3).unwrap_or_default(),
        });
    }

    Ok(result)
}

fn clear_notifications_db(path: &Path) -> Result<(), String> {
    let conn = Connection::open(path).map_err(|e| format!("failed to open notification db: {e}"))?;
    conn.execute("DELETE FROM notifications", [])
        .map_err(|e| format!("failed to clear notifications: {e}"))?;
    Ok(())
}

fn apply_hibernate_script(handle: &tauri::AppHandle, label: &str) {
    if let Some(wv) = handle.get_webview(label) {
        let _ = wv.eval(web_scripts::hibernate_script());
    }
}

fn destroy_inactive_webview(handle: &tauri::AppHandle, label: &str) {
    if let Some(wv) = handle.get_webview(label) {
        let _ = wv.close();
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

fn schedule_hibernation(handle: &tauri::AppHandle, label: String) {
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

fn layout_offsets(logical_width: f64) -> (f64, f64, f64) {
    if logical_width < 640.0 {
        (56.0, 56.0, 0.0)
    } else {
        (64.0, 56.0, 24.0)
    }
}

fn compute_webview_bounds(
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

    fn parse_external_url(raw: &str) -> Result<Url, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("empty url".to_string());
        }

        Url::parse(trimmed)
            .or_else(|_| Url::parse(&format!("https://{trimmed}")))
            .map_err(|_| format!("invalid url: {trimmed}"))
    }

fn set_webview_hibernation(handle: &tauri::AppHandle, active_id: Option<&str>) {
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
            let _ = wv.show();
            let _ = wv.eval(web_scripts::wake_script());
        }
    }
}

#[tauri::command]
fn get_resource_usage() -> ResourceUsage {
    let (cpu_percent, ram_mb) = metrics::get_resource_usage_cross_platform(std::process::id());
    ResourceUsage {
        cpu_percent: (cpu_percent * 10.0).round() / 10.0,
        ram_mb,
    }
}

#[tauri::command]
fn set_safe_mode(state: State<'_, AppState>, enabled: bool) {
    let mut safe_mode = state.safe_mode.lock().expect("safe_mode lock poisoned");
    *safe_mode = enabled;
}

#[tauri::command]
fn report_webview_error(
    app: tauri::AppHandle,
    app_id: String,
    webview_id: Option<String>,
    category: String,
    message: String,
    detail: Option<String>,
) {
    emit_diagnostic(
        &app,
        "warn",
        &category,
        &app_id,
        webview_id.as_deref(),
        &message,
        detail,
    );
}

#[tauri::command]
fn get_notifications(state: State<'_, AppState>, limit: Option<u32>) -> Result<Vec<VesselNotification>, String> {
    list_notifications(&state.db_path, limit.unwrap_or(200).min(500))
}

#[tauri::command]
fn clear_notifications(state: State<'_, AppState>) -> Result<(), String> {
    clear_notifications_db(&state.db_path)
}

#[tauri::command]
fn forward_notification(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    app_id: String,
    title: String,
    body: String,
) {
    let note = VesselNotification {
        app_id,
        title,
        body,
        time: Local::now().format("%I:%M %p").to_string(),
    };

    let _ = insert_notification(&state.db_path, &note);
    let _ = app.emit("vessel-notification", &note);
}

#[tauri::command]
fn request_new_tab(app: tauri::AppHandle, app_id: String, url: String, title: Option<String>) {
    let payload = NewTabRequest { app_id, url, title };
    let _ = app.emit("vessel-open-tab", payload);
}

#[tauri::command]
fn close_webview(handle: tauri::AppHandle, state: State<'_, AppState>, id: String) {
    if let Some(existing_wv) = handle.get_webview(&id) {
        let _ = existing_wv.close();
    }

    {
        let mut profiles = state
            .active_profiles
            .lock()
            .expect("active_profiles lock poisoned");
        profiles.remove(&id);
    }
    {
        let mut sleep = state
            .app_sleep_seconds
            .lock()
            .expect("app_sleep_seconds lock poisoned");
        sleep.remove(&id);
    }
    {
        let mut tokens = state
            .hibernate_tokens
            .lock()
            .expect("hibernate_tokens lock poisoned");
        tokens.remove(&id);
    }

    let mut active = state
        .active_webview
        .lock()
        .expect("active_webview lock poisoned");
    if active.as_deref() == Some(id.as_str()) {
        *active = None;
    }
}

#[tauri::command]
fn hide_all_webviews(handle: tauri::AppHandle, state: State<'_, AppState>) {
    let mut active = state
        .active_webview
        .lock()
        .expect("active_webview lock poisoned");
    *active = None;

    set_webview_hibernation(&handle, None);
}

#[tauri::command]
fn open_app(
    window: tauri::Window,
    handle: tauri::AppHandle,
    state: State<'_, AppState>,
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
    let root_app_id = app_id.unwrap_or_else(|| id.clone());
    let app_key = sanitize_segment(&root_app_id, "app");
    let profile_name = sanitize_segment(profile.as_deref().unwrap_or("default"), "default");
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
        profiles.get(&id).map(|saved| saved != &expected_profile).unwrap_or(false)
    };

    if should_recreate {
        if let Some(existing_wv) = handle.get_webview(&id) {
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
        if let Some((_, _, rect)) = compute_webview_bounds(&window) {
            let _ = existing_wv.set_bounds(rect);
        }
        let _ = existing_wv.show();
        let _ = existing_wv.set_focus();
        let _ = existing_wv.eval(web_scripts::wake_script());

        if let Err(err) = existing_wv.eval(&notification_script) {
            emit_diagnostic(
                &handle,
                "warn",
                "ipc",
                &root_app_id,
                Some(&id),
                "notification hijack injection failed",
                Some(err.to_string()),
            );
        }

        if let Err(err) = existing_wv.eval(&tab_script) {
            emit_diagnostic(
                &handle,
                "warn",
                "ipc",
                &root_app_id,
                Some(&id),
                "tab bridge injection failed",
                Some(err.to_string()),
            );
        }

        if let Some(css_code) = css {
            if !css_code.trim().is_empty() {
                if let Err(err) = security::validate_css_payload(&css_code) {
                    emit_diagnostic(
                        &handle,
                        "warn",
                        "security",
                        &root_app_id,
                        Some(&id),
                        "base css blocked",
                        Some(err),
                    );
                } else {
                    let css_script = web_scripts::upsert_style_script("vessel-base-css", &css_code);
                    if let Err(err) = existing_wv.eval(&css_script) {
                        emit_diagnostic(
                            &handle,
                            "warn",
                            "injection",
                            &root_app_id,
                            Some(&id),
                            "base css injection failed",
                            Some(err.to_string()),
                        );
                    }
                }
            }
        }

        if allow_custom_injection {
            if let Some(custom_css_code) = custom_css {
                if !custom_css_code.trim().is_empty() {
                    if let Err(err) = security::validate_css_payload(&custom_css_code) {
                        emit_diagnostic(
                            &handle,
                            "warn",
                            "security",
                            &root_app_id,
                            Some(&id),
                            "custom css blocked",
                            Some(err),
                        );
                    } else {
                        let css_script = web_scripts::upsert_style_script("vessel-custom-css", &custom_css_code);
                        if let Err(err) = existing_wv.eval(&css_script) {
                            emit_diagnostic(
                                &handle,
                                "warn",
                                "injection",
                                &root_app_id,
                                Some(&id),
                                "custom css injection failed",
                                Some(err.to_string()),
                            );
                        }
                    }
                }
            }
        } else if custom_css.as_ref().is_some_and(|val| !val.trim().is_empty()) {
            emit_diagnostic(
                &handle,
                "warn",
                "security",
                &root_app_id,
                Some(&id),
                "custom css blocked by allowlist",
                Some("current domain does not match configured allowlist".to_string()),
            );
        }

        if !safe_mode_enabled {
            if let Some(js_code) = js {
                if !js_code.trim().is_empty() {
                    if let Err(err) = security::validate_js_payload(&js_code) {
                        emit_diagnostic(
                            &handle,
                            "warn",
                            "security",
                            &root_app_id,
                            Some(&id),
                            "js payload blocked",
                            Some(err),
                        );
                    } else {
                        let runtime_script = web_scripts::upsert_runtime_js_script("vessel-runtime-js", &js_code);
                        if let Err(err) = existing_wv.eval(&runtime_script) {
                            emit_diagnostic(
                                &handle,
                                "warn",
                                "injection",
                                &root_app_id,
                                Some(&id),
                                "js injection failed",
                                Some(err.to_string()),
                            );
                        }
                    }
                }
            }

            if let Some(custom_js_code) = custom_js {
                if !custom_js_code.trim().is_empty() {
                    if let Err(err) = security::validate_js_payload(&custom_js_code) {
                        emit_diagnostic(
                            &handle,
                            "warn",
                            "security",
                            &root_app_id,
                            Some(&id),
                            "custom js payload blocked",
                            Some(err),
                        );
                    } else if allow_custom_injection {
                        let runtime_script = web_scripts::upsert_runtime_js_script("vessel-custom-js", &custom_js_code);
                        if let Err(err) = existing_wv.eval(&runtime_script) {
                            emit_diagnostic(
                                &handle,
                                "warn",
                                "injection",
                                &root_app_id,
                                Some(&id),
                                "custom js injection failed",
                                Some(err.to_string()),
                            );
                        }
                    } else {
                        emit_diagnostic(
                            &handle,
                            "warn",
                            "security",
                            &root_app_id,
                            Some(&id),
                            "custom js blocked by allowlist",
                            Some("current domain does not match configured allowlist".to_string()),
                        );
                    }
                }
            }
        } else if js.as_ref().is_some_and(|val| !val.trim().is_empty())
            || custom_js.as_ref().is_some_and(|val| !val.trim().is_empty())
        {
            emit_diagnostic(
                &handle,
                "info",
                "security",
                &root_app_id,
                Some(&id),
                "safe mode skipped runtime js",
                None,
            );
        }

        let mut profiles = state.active_profiles.lock().expect("active_profiles lock poisoned");
        profiles.insert(id, expected_profile);
        return Ok(());
    }

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
    builder = builder.data_directory(session_dir);

        builder = builder.initialization_script(web_scripts::stealth_injection_script());
        builder = builder.initialization_script(&notification_script);
        builder = builder.initialization_script(&tab_script);

    if let Some(css_code) = css {
        if !css_code.trim().is_empty() {
            if security::validate_css_payload(&css_code).is_ok() {
                let css_script = web_scripts::upsert_style_script("vessel-base-css", &css_code);
                builder = builder.initialization_script(&css_script);
            }
        }
    }

    if allow_custom_injection {
        if let Some(custom_css_code) = custom_css {
            if !custom_css_code.trim().is_empty() && security::validate_css_payload(&custom_css_code).is_ok() {
                let custom_css_script = web_scripts::upsert_style_script("vessel-custom-css", &custom_css_code);
                builder = builder.initialization_script(&custom_css_script);
            }
        }
    }

    if !safe_mode_enabled {
        if let Some(js_code) = js {
            if !js_code.trim().is_empty() && security::validate_js_payload(&js_code).is_ok() {
                let js_script = web_scripts::upsert_runtime_js_script("vessel-runtime-js", &js_code);
                builder = builder.initialization_script(&js_script);
            }
        }
        if let Some(custom_js_code) = custom_js {
            if !custom_js_code.trim().is_empty()
                && allow_custom_injection
                && security::validate_js_payload(&custom_js_code).is_ok()
            {
                let js_script = web_scripts::upsert_runtime_js_script("vessel-custom-js", &custom_js_code);
                builder = builder.initialization_script(&js_script);
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let db_path = notification_db_path(&app.handle());
            if let Err(e) = ensure_db(&db_path) {
                return Err(Box::<dyn std::error::Error>::from(e));
            }

            let sessions_root = sessions_root_path(&app.handle());
            let _ = fs::create_dir_all(&sessions_root);

            app.manage(AppState {
                db_path,
                sessions_root,
                active_profiles: Mutex::new(HashMap::new()),
                app_sleep_seconds: Mutex::new(HashMap::new()),
                active_webview: Mutex::new(None),
                hibernate_tokens: Mutex::new(HashMap::new()),
                safe_mode: Mutex::new(false),
            });

            let Some(main_webview_win) = app.get_webview_window("main") else {
                return Err(Box::<dyn std::error::Error>::from("main window not found"));
            };
            let main_win = main_webview_win.as_ref().window().clone();
            let app_handle = app.handle().clone();
            let main_win_clone = main_win.clone();

            main_win.on_window_event(move |event| {
                if let WindowEvent::Resized(size) = event {
                    let scale = main_win_clone.scale_factor().unwrap_or(1.0);
                    let logical_width = (size.width as f64) / scale;
                    let (sidebar_w, header_h, footer_h) = layout_offsets(logical_width);

                    let sidebar_width = (sidebar_w * scale) as u32;
                    let header_height = (header_h * scale) as u32;
                    let footer_height = (footer_h * scale) as u32;

                    for (label, wv) in app_handle.webviews() {
                        if label != "main" {
                            let _ = wv.set_bounds(Rect {
                                position: PhysicalPosition::new(sidebar_width as i32, header_height as i32)
                                    .into(),
                                size: PhysicalSize::new(
                                    size.width.saturating_sub(sidebar_width),
                                    size.height.saturating_sub(header_height + footer_height),
                                )
                                .into(),
                            });
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_app,
            forward_notification,
            request_new_tab,
            close_webview,
            hide_all_webviews,
            get_notifications,
            clear_notifications,
            get_resource_usage,
            set_safe_mode,
            report_webview_error
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vessel");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_segment_strips_invalid_chars() {
        assert_eq!(sanitize_segment("  Work/Profile!  ", "fallback"), "Work-Profile");
    }

    #[test]
    fn parse_external_url_adds_https_when_missing() {
        let parsed = parse_external_url("example.com").expect("url should parse");
        assert_eq!(parsed.scheme(), "https");
        assert_eq!(parsed.host_str(), Some("example.com"));
    }

    #[test]
    fn notification_payload_serializes_camel_case() {
        let payload = VesselNotification {
            app_id: "app-a".to_string(),
            title: "Title".to_string(),
            body: "Body".to_string(),
            time: "10:00 AM".to_string(),
        };
        let value = serde_json::to_value(payload).expect("serialize notification");
        assert!(value.get("appId").is_some());
    }

    #[test]
    fn new_tab_request_serializes_camel_case() {
        let payload = NewTabRequest {
            app_id: "app-a".to_string(),
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
        };
        let value = serde_json::to_value(payload).expect("serialize request");
        assert!(value.get("appId").is_some());
    }
}
