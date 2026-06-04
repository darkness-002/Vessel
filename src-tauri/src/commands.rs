use tauri::{Emitter, Manager, State};
use chrono::Local;
use crate::state::{AppState, VesselNotification, MAX_NOTIFICATIONS_PER_MINUTE};
use crate::{db, webview};
use crate::metrics;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUsage {
    pub cpu_percent: f32,
    pub ram_mb: u64,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NewTabRequest {
    pub app_id: String,
    pub url: String,
    pub title: Option<String>,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEvent {
    pub level: String,
    pub category: String,
    pub app_id: String,
    pub webview_id: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    pub time: String,
}

pub fn emit_diagnostic(
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

#[tauri::command]
pub async fn get_resource_usage() -> Result<ResourceUsage, String> {
    let (cpu_percent, ram_mb) = metrics::get_resource_usage_cross_platform(std::process::id());
    Ok(ResourceUsage {
        cpu_percent: (cpu_percent * 10.0).round() / 10.0,
        ram_mb,
    })
}

#[tauri::command]
pub fn set_safe_mode(state: State<'_, AppState>, enabled: bool) {
    let mut safe_mode = state.safe_mode.lock().expect("safe_mode lock poisoned");
    *safe_mode = enabled;
}

#[tauri::command(rename_all = "camelCase")]
pub fn report_webview_error(
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
pub fn get_encryption_key() -> Result<String, String> {
    db::get_encryption_key()
}

#[tauri::command]
pub async fn get_notifications(state: State<'_, AppState>, limit: Option<u32>) -> Result<Vec<VesselNotification>, String> {
    db::list_notifications(&state.db_path, limit.unwrap_or(200).min(500))
}

#[tauri::command]
pub async fn clear_notifications(state: State<'_, AppState>) -> Result<(), String> {
    db::clear_notifications_db(&state.db_path)
}

#[tauri::command(rename_all = "camelCase")]
pub fn forward_notification(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    app_id: String,
    title: String,
    body: String,
) {
    let now = Local::now().timestamp();
    {
        let mut limits = state.notification_rate_limit.lock().expect("notification_rate_limit lock poisoned");
        let (count, timestamp) = limits.entry(app_id.clone()).or_insert((0, now));
        
        if now - *timestamp > 60 {
            *count = 1;
            *timestamp = now;
        } else {
            *count += 1;
        }

        if *count > MAX_NOTIFICATIONS_PER_MINUTE {
            return;
        }
    }

    let note = VesselNotification {
        app_id,
        title,
        body,
        time: Local::now().format("%I:%M %p").to_string(),
    };

    let _ = db::insert_notification(&state.db_path, &note);
    let _ = app.emit("vessel-notification", &note);
}

#[tauri::command(rename_all = "camelCase")]
pub fn request_new_tab(app: tauri::AppHandle, app_id: String, url: String, title: Option<String>) {
    let payload = NewTabRequest { app_id, url, title };
    let _ = app.emit("vessel-open-tab", payload);
}

#[tauri::command(rename_all = "camelCase")]
pub async fn close_webview(handle: tauri::AppHandle, id: String) -> Result<(), String> {
    println!("Command: close_webview(id: {})", id);
    webview::destroy_inactive_webview(&handle, &id);
    
    let state = handle.state::<AppState>();
    let mut active = state
        .active_webview
        .lock()
        .expect("active_webview lock poisoned");
    if active.as_deref() == Some(id.as_str()) {
        println!("  Resetting active_webview from {}", id);
        *active = None;
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_all_webviews(handle: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut active = state
        .active_webview
        .lock()
        .expect("active_webview lock poisoned");
    *active = None;

    webview::set_webview_hibernation(&handle, None);
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn open_app(
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
    println!("Command: open_app(id: {}, app_id: {:?})", id, app_id);
    webview::orchestrate_app_webview(
        window,
        handle,
        &state,
        id,
        url,
        app_id,
        profile,
        css,
        custom_css,
        js,
        custom_js,
        injection_allowlist,
        idle_sleep_seconds,
    )
}

#[tauri::command(rename_all = "camelCase")]
pub async fn delete_app_session(state: State<'_, AppState>, app_id: String) -> Result<(), String> {
    println!("Command: delete_app_session(app_id: {})", app_id);
    let app_key = crate::state::sanitize_segment(&app_id, "app");
    let path = state.sessions_root.join(&app_key);
    println!("  Deleting session path: {:?}", path);
    
    if std::fs::metadata(&path).is_ok() {
        let mut last_err = String::new();
        // Be more patient: 10 retries with 500ms backoff = up to 5 seconds
        for i in 0..10 {
            if i > 0 {
                println!("  Retry {}/10...", i);
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            match std::fs::remove_dir_all(&path) {
                Ok(_) => {
                    println!("  Successfully deleted session data.");
                    return Ok(());
                }
                Err(e) => {
                    last_err = format!("{e}");
                    println!("  Attempt {} failed: {}", i + 1, last_err);
                }
            }
        }
        return Err(format!("failed to delete session data after 10 retries: {}", last_err));
    } else {
        println!("  Path does not exist, skipping deletion.");
    }
    Ok(())
}
