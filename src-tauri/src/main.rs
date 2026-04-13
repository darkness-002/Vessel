#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    Manager, PhysicalPosition, PhysicalSize, LogicalPosition, LogicalSize, 
    webview::WebviewBuilder, WebviewUrl, WindowEvent, Emitter
};

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
fn hide_all_webviews(handle: tauri::AppHandle) {
    for (label, wv) in handle.webviews() {
        if label != "main" { let _ = wv.hide(); }
    }
}

#[tauri::command]
async fn open_app(handle: tauri::AppHandle, id: String, url: String, css: Option<String>, js: Option<String>) {
    for (label, wv) in handle.webviews() {
        if label != "main" && label != id { let _ = wv.hide(); }
    }

    if let Some(existing_wv) = handle.get_webview(&id) {
        let _ = existing_wv.show();
        let _ = existing_wv.set_focus();
        if let Some(css_code) = css {
            if !css_code.is_empty() {
                let css_script = format!("const style = document.createElement('style'); style.textContent = `{}`; document.head.appendChild(style);", css_code);
                let _ = existing_wv.eval(&css_script);
            }
        }
    } else {
        let main_webview_win = handle.get_webview_window("main").unwrap();
        let main_win = main_webview_win.as_ref().window(); 
        
        let scale = main_win.scale_factor().unwrap_or(1.0);
        let size = main_win.inner_size().unwrap();
        let logical_width = (size.width as f64) / scale;
        let logical_height = (size.height as f64) / scale;

        let mut builder = WebviewBuilder::new(&id, WebviewUrl::External(url.parse().unwrap()));

        let hijack_script = format!(
            r#"window.originalNotification = window.Notification; window.Notification = class {{ constructor(title, options) {{ window.__TAURI__.core.invoke('forward_notification', {{ appId: '{}', title: title, body: options ? (options.body || '') : '' }}); }} static get permission() {{ return 'granted'; }} static requestPermission() {{ return Promise.resolve('granted'); }} }};"#,
            id
        );
        builder = builder.initialization_script(&hijack_script);

        if let Some(css_code) = css {
            if !css_code.is_empty() {
                let css_script = format!("window.addEventListener('DOMContentLoaded', () => {{ const style = document.createElement('style'); style.textContent = `{}`; document.head.appendChild(style); }});", css_code);
                builder = builder.initialization_script(&css_script);
            }
        }

        if let Some(js_code) = js {
            if !js_code.is_empty() { builder = builder.initialization_script(&js_code); }
        }

        // NEW GEOMETRY: Left 64px, Top 48px, Height offset -72px (Header + Footer)
        let _webview = main_win.add_child(
            builder,
            LogicalPosition::new(64.0, 48.0), 
            LogicalSize::new(logical_width - 64.0, logical_height - 72.0),
        ).unwrap();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let main_webview_win = app.get_webview_window("main").unwrap();
            let main_win = main_webview_win.as_ref().window().clone();
            let app_handle = app.handle().clone();
            let main_win_clone = main_win.clone();
            
            main_win.on_window_event(move |event| {
                if let WindowEvent::Resized(size) = event {
                    let scale = main_win_clone.scale_factor().unwrap_or(1.0);
                    
                    let sidebar_width = (64.0 * scale) as u32; 
                    let header_height = (48.0 * scale) as u32;
                    let footer_height = (24.0 * scale) as u32;
                    
                    for (label, wv) in app_handle.webviews() {
                        if label != "main" {
                            let _ = wv.set_bounds(tauri::Rect {
                                position: PhysicalPosition::new(sidebar_width as i32, header_height as i32).into(),
                                size: PhysicalSize::new(
                                    size.width.saturating_sub(sidebar_width), 
                                    size.height.saturating_sub(header_height + footer_height)
                                ).into(),
                            });
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![open_app, forward_notification, hide_all_webviews])
        .run(tauri::generate_context!())
        .expect("error while running Vessel");
}