#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    Manager, PhysicalPosition, PhysicalSize, LogicalPosition, LogicalSize, 
    webview::WebviewBuilder, WebviewUrl, WindowEvent, Emitter
};
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Instant, Duration};

struct AppState {
    last_active: Mutex<HashMap<String, Instant>>,
    active_app: Mutex<String>,
}

// The receiver for hijacked notifications
#[tauri::command]
fn forward_notification(app: tauri::AppHandle, app_id: String, title: String, body: String) {
    let _ = app.emit("vessel-notification", serde_json::json!({
        "appId": app_id,
        "title": title,
        "body": body,
        "time": chrono::Local::now().format("%I:%M %p").to_string()
    }));
}

#[tauri::command]
async fn open_app(
    handle: tauri::AppHandle, 
    state: tauri::State<'_, AppState>,
    id: String, 
    url: String, 
    css: Option<String>, 
    js: Option<String>,
    profile: Option<String>
) -> Result<(), String> {
    *state.active_app.lock().unwrap() = id.clone();
    state.last_active.lock().unwrap().insert(id.clone(), Instant::now());

    // Hide all other webviews
    for (label, wv) in handle.webviews() {
        if label != "main" && label != id {
            let _ = wv.hide();
        }
    }

    // HOT-SWAP LOGIC: If the app is already open
    if let Some(existing_wv) = handle.get_webview(&id) {
        let _ = existing_wv.show();
        let _ = existing_wv.set_focus();
        
        // Instantly inject new CSS if settings were changed
        if let Some(css_code) = css {
            if !css_code.is_empty() {
                let css_script = format!(
                    "const style = document.createElement('style'); style.textContent = `{}`; document.head.appendChild(style);",
                    css_code
                );
                let _ = existing_wv.eval(&css_script);
            }
        }
    } else {
        // If the app is NOT open, create it
        let main_webview_win = handle.get_webview_window("main").unwrap();
        let main_win = main_webview_win.as_ref().window(); 
        
        let scale = main_win.scale_factor().unwrap_or(1.0);
        let size = main_win.inner_size().unwrap();
        let logical_width = (size.width as f64) / scale;
        let logical_height = (size.height as f64) / scale;

        let mut builder = WebviewBuilder::new(&id, WebviewUrl::External(url.parse().unwrap()));

        if let Some(profile_name) = profile {
            if !profile_name.is_empty() && profile_name.to_lowercase() != "default" {
                if let Ok(mut data_dir) = handle.path().app_data_dir() {
                    data_dir.push("profiles");
                    data_dir.push(profile_name);
                    // Ensure the directory exists to prevent webview crash
                    let _ = std::fs::create_dir_all(&data_dir);
                    builder = builder.data_directory(data_dir);
                }
            }
        }

        // Notification Hijacker Script
        let hijack_script = format!(
            r#"
            window.originalNotification = window.Notification;
            window.Notification = class {{
                constructor(title, options) {{
                    window.__TAURI__.core.invoke('forward_notification', {{
                        appId: '{}',
                        title: title,
                        body: options ? (options.body || '') : ''
                    }});
                }}
                static get permission() {{ return 'granted'; }}
                static requestPermission() {{ return Promise.resolve('granted'); }}
            }};
            "#,
            id
        );
        builder = builder.initialization_script(&hijack_script);

        // Inject Universal CSS on boot
        if let Some(css_code) = css {
            if !css_code.is_empty() {
                let css_script = format!(
                    "window.addEventListener('DOMContentLoaded', () => {{ const style = document.createElement('style'); style.textContent = `{}`; document.head.appendChild(style); }});",
                    css_code
                );
                builder = builder.initialization_script(&css_script);
            }
        }

        // Inject Site-Specific JS on boot
        if let Some(js_code) = js {
            if !js_code.is_empty() {
                builder = builder.initialization_script(&js_code);
            }
        }

        let _webview = main_win.add_child(
            builder,
            LogicalPosition::new(70.0, 0.0), 
            LogicalSize::new(logical_width - 70.0, logical_height),
        ).unwrap();
    }
    
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            last_active: Mutex::new(HashMap::new()),
            active_app: Mutex::new(String::new()),
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Phase 6: Sleep Engine Background Thread
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(60));
                    
                    let state = app_handle.state::<AppState>();
                    let active_app = state.active_app.lock().unwrap().clone();
                    let mut last_active = state.last_active.lock().unwrap();
                    
                    let mut to_drop = Vec::new();
                    for (id, time) in last_active.iter() {
                        // Drop webviews inactive for more than 30 minutes
                        if *id != active_app && time.elapsed() > Duration::from_secs(30 * 60) {
                            to_drop.push(id.clone());
                        }
                    }
                    
                    for id in to_drop {
                        if let Some(wv) = app_handle.get_webview(&id) {
                            let _ = wv.close();
                            println!("Sleep Engine: Dropped inactive webview -> {}", id);
                        }
                        last_active.remove(&id);
                    }
                }
            });

            let main_webview_win = app.get_webview_window("main").unwrap();
            let main_win = main_webview_win.as_ref().window().clone();
            let app_handle2 = app.handle().clone();
            let main_win_clone = main_win.clone();
            
            // Phase 5: Prevent Close, Hide Instead
            main_win.on_window_event(move |event| {
                match event {
                    WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = main_win_clone.hide();
                    }
                    WindowEvent::Resized(size) => {
                        let scale = main_win_clone.scale_factor().unwrap_or(1.0);
                        let sidebar_width = (70.0 * scale) as u32; 
                        
                        for (label, wv) in app_handle2.webviews() {
                            if label != "main" {
                                let _ = wv.set_bounds(tauri::Rect {
                                    position: PhysicalPosition::new(sidebar_width as i32, 0).into(),
                                    size: PhysicalSize::new(size.width.saturating_sub(sidebar_width), size.height).into(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            });

            // Phase 5: Tray Icon Setup
            if let Some(icon) = app.default_window_icon() {
                let _tray = TrayIconBuilder::new()
                    .icon(icon.clone())
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                            let app = tray.app_handle();
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    })
                    .build(app);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![open_app, forward_notification])
        .run(tauri::generate_context!())
        .expect("error while running Vessel");
}