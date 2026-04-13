#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
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
        let _ = wv.eval(hibernate_script());
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

fn hibernate_script() -> &'static str {
    r#"
    if (!window.__vesselHibernate) {
      window.__vesselHibernate = true;
      document.querySelectorAll('video,audio').forEach((m) => {
        try {
          if (!m.paused) {
            m.dataset.vesselWasPlaying = '1';
            m.pause();
          }
        } catch (_) {}
      });
      document.documentElement.style.setProperty('caret-color', 'transparent');
    }
    "#
}

fn wake_script() -> &'static str {
    r#"
    if (window.__vesselHibernate) {
      window.__vesselHibernate = false;
      document.querySelectorAll('video,audio').forEach((m) => {
        try {
          if (m.dataset.vesselWasPlaying === '1') {
            m.play().catch(() => {});
            delete m.dataset.vesselWasPlaying;
          }
        } catch (_) {}
      });
      document.documentElement.style.removeProperty('caret-color');
    }
    "#
}

fn notification_hijack_script(app_id: &str) -> String {
        format!(
                r#"window.originalNotification = window.Notification; window.Notification = class {{ constructor(title, options) {{ window.__TAURI__.core.invoke('forward_notification', {{ appId: '{}', title: title, body: options ? (options.body || '') : '' }}); }} static get permission() {{ return 'granted'; }} static requestPermission() {{ return Promise.resolve('granted'); }} }};"#,
                app_id
        )
}

fn stealth_injection_script() -> &'static str {
    r#"
    (function() {
        if (window.__vesselStealthInstalled) return;
        window.__vesselStealthInstalled = true;

        // Mask webdriver detection
        try {
            Object.defineProperty(navigator, 'webdriver', {
                get: () => undefined,  
                set: () => {},
                configurable: true
            });
        } catch (e) {}

        // Mock chrome object if undefined or incomplete
        if (!window.chrome) {
            window.chrome = {};
        }
        if (!window.chrome.runtime) {
            window.chrome.runtime = {};
        }

        // Override common fingerprinting properties
        try {
            Object.defineProperty(navigator, 'permissions', {
                get: () => ({
                    query: () => Promise.resolve({ state: 'granted' })
                }),
                configurable: true
            });
        } catch (e) {}

        // Mask automation flags
        try {
            delete navigator.__proto__.webdriver;
        } catch (e) {}
        try {
            if (window.__TAURI_INTERNALS__) {
                window.__VESSEL_EMBEDDED = true;
            }
        } catch (e) {}
    })();
    "#
}

fn escape_css_for_js(css: &str) -> String {
    // Use serde_json to safely escape the CSS string for JavaScript
    // This handles all special characters: backticks, quotes, newlines, etc.
    serde_json::to_string(css).unwrap_or_else(|_| String::from("\"\""))
}

fn new_tab_bridge_script(app_id: &str) -> String {
        r#"
        (function() {
            if (window.__vesselTabBridgeInstalled) return;
            window.__vesselTabBridgeInstalled = true;

            const __absoluteUrl = (candidate) => {
                try {
                    if (!candidate) return null;
                    return new URL(String(candidate), window.location.href).href;
                } catch (_err) {
                    return null;
                }
            };

            const __invoke = (command, payload) => {
                try {
                    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
                        return window.__TAURI__.core.invoke(command, payload);
                    }
                    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
                        return window.__TAURI_INTERNALS__.invoke(command, payload);
                    }
                } catch (_err) {}
                return Promise.resolve(null);
            };

            const __vesselOpenTab = (targetUrl, targetTitle) => {
                const resolved = __absoluteUrl(targetUrl);
                if (!resolved) return;
                __invoke('request_new_tab', {
                    appId: '__APP_ID__',
                    url: resolved,
                    title: targetTitle ? String(targetTitle) : null
                });
            };

            const originalOpen = window.open;
            window.open = function(url, name, specs) {
                if (url) {
                    __vesselOpenTab(url, name || null);
                    return null;
                }
                return originalOpen ? originalOpen.apply(window, arguments) : null;
            };

            const findAnchor = (e) => {
                if (e.target && e.target.closest) {
                    const viaClosest = e.target.closest('a[href]');
                    if (viaClosest) return viaClosest;
                }

                if (typeof e.composedPath === 'function') {
                    const path = e.composedPath();
                    for (const node of path) {
                        if (!node || typeof node !== 'object') continue;
                        if (node.tagName === 'A' && node.href) return node;
                    }
                }
                return null;
            };

            document.addEventListener('click', function(e) {
                const anchor = findAnchor(e);
                if (!anchor || !anchor.href) return;

                const isModifiedClick = e.metaKey || e.ctrlKey;
                const opensBlank = anchor.target === '_blank';
                if (isModifiedClick || opensBlank) {
                    e.preventDefault();
                    __vesselOpenTab(anchor.href, anchor.title || anchor.textContent || null);
                }
            }, true);

            document.addEventListener('auxclick', function(e) {
                if (e.button !== 1) return;
                const anchor = findAnchor(e);
                if (!anchor || !anchor.href) return;
                e.preventDefault();
                __vesselOpenTab(anchor.href, anchor.title || anchor.textContent || null);
            }, true);
        })();
        "#
        .replace("__APP_ID__", app_id)
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
            let _ = wv.eval(wake_script());
        }
    }
}

#[tauri::command]
fn get_resource_usage() -> ResourceUsage {
    let root_pid = std::process::id();
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid=,%cpu=,rss="])
        .output();

    if let Ok(raw) = output {
        let stdout = String::from_utf8_lossy(&raw.stdout);
        let mut proc_rows: Vec<(u32, u32, f32, u64)> = Vec::new();

        for line in stdout.lines() {
            let mut parts = line.split_whitespace();
            let pid = parts.next().and_then(|v| v.parse::<u32>().ok());
            let ppid = parts.next().and_then(|v| v.parse::<u32>().ok());
            let cpu = parts.next().and_then(|v| v.parse::<f32>().ok());
            let rss_kb = parts.next().and_then(|v| v.parse::<u64>().ok());

            if let (Some(pid), Some(ppid), Some(cpu), Some(rss_kb)) = (pid, ppid, cpu, rss_kb) {
                proc_rows.push((pid, ppid, cpu, rss_kb));
            }
        }

        let mut included: std::collections::HashSet<u32> = std::collections::HashSet::new();
        included.insert(root_pid);

        let mut changed = true;
        while changed {
            changed = false;
            for (pid, ppid, _, _) in &proc_rows {
                if included.contains(ppid) && !included.contains(pid) {
                    included.insert(*pid);
                    changed = true;
                }
            }
        }

        let mut total_cpu = 0.0_f32;
        let mut total_rss_kb = 0_u64;

        for (pid, _ppid, cpu, rss_kb) in &proc_rows {
            if included.contains(pid) {
                total_cpu += *cpu;
                total_rss_kb += *rss_kb;
            }
        }

        return ResourceUsage {
            cpu_percent: (total_cpu * 10.0).round() / 10.0,
            ram_mb: total_rss_kb / 1024,
        };
    }

    ResourceUsage {
        cpu_percent: 0.0,
        ram_mb: 0,
    }
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
    idle_sleep_seconds: Option<u64>,
) -> Result<(), String> {
    let root_app_id = app_id.unwrap_or_else(|| id.clone());
    let app_key = sanitize_segment(&root_app_id, "app");
    let profile_name = sanitize_segment(profile.as_deref().unwrap_or("default"), "default");
    let notification_script = notification_hijack_script(&root_app_id);
    let tab_script = new_tab_bridge_script(&root_app_id);

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
        let _ = existing_wv.eval(wake_script());
        let _ = existing_wv.eval(&notification_script);
        let _ = existing_wv.eval(&tab_script);

        if let Some(css_code) = css {
            if !css_code.trim().is_empty() {
                let escaped_css = escape_css_for_js(&css_code);
                let css_script = format!(
                    "const style = document.createElement('style'); style.textContent = {}; document.head.appendChild(style);",
                    escaped_css
                );
                let _ = existing_wv.eval(&css_script);
            }
        }

        if let Some(custom_css_code) = custom_css {
            if !custom_css_code.trim().is_empty() {
                let escaped_css = escape_css_for_js(&custom_css_code);
                let css_script = format!(
                    "const style = document.createElement('style'); style.id = 'vessel-custom-css'; style.textContent = {}; document.head.appendChild(style);",
                    escaped_css
                );
                let _ = existing_wv.eval(&css_script);
            }
        }

        let mut profiles = state.active_profiles.lock().expect("active_profiles lock poisoned");
        profiles.insert(id, expected_profile);
        return Ok(());
    }

    let Some((logical_pos, logical_size, _)) = compute_webview_bounds(&window) else {
        return Err("unable to compute webview bounds".to_string());
    };

    let external_url = parse_external_url(&url)?;

    let mut builder = WebviewBuilder::new(&id, WebviewUrl::External(external_url))
        .user_agent(get_stealth_user_agent());

    let session_dir = state
        .sessions_root
        .join(&app_key)
        .join(&profile_name);
    let _ = fs::create_dir_all(&session_dir);
    builder = builder.data_directory(session_dir);

        builder = builder.initialization_script(stealth_injection_script());
        builder = builder.initialization_script(&notification_script);
        builder = builder.initialization_script(&tab_script);

    if let Some(css_code) = css {
        if !css_code.trim().is_empty() {
            let escaped_css = escape_css_for_js(&css_code);
            let css_script = format!(
                "const style = document.createElement('style'); style.textContent = {}; document.head.appendChild(style);",
                escaped_css
            );
            builder = builder.initialization_script(&css_script);
        }
    }

    if let Some(custom_css_code) = custom_css {
        if !custom_css_code.trim().is_empty() {
            let escaped_css = escape_css_for_js(&custom_css_code);
            let custom_css_script = format!(
                "const style = document.createElement('style'); style.id = 'vessel-custom-css'; style.textContent = {}; document.head.appendChild(style);",
                escaped_css
            );
            builder = builder.initialization_script(&custom_css_script);
        }
    }

    if let Some(js_code) = js {
        if !js_code.trim().is_empty() {
            builder = builder.initialization_script(&js_code);
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
            get_resource_usage
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vessel");
}
