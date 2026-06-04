mod db;
mod state;
mod commands;
mod webview;
mod metrics;
mod security;
mod web_scripts;

use std::fs;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Manager, PhysicalPosition, PhysicalSize, Rect, WindowEvent};

use crate::state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_stronghold::Builder::new(|password| {
            password.as_bytes()[0..32].to_vec()
        }).build())
        .setup(|app| {
            let db_path = db::notification_db_path(&app.handle());
            if let Err(e) = db::ensure_db(&db_path) {
                return Err(Box::<dyn std::error::Error>::from(e));
            }

            let sessions_root = db::sessions_root_path(&app.handle());
            let _ = fs::create_dir_all(&sessions_root);

            app.manage(AppState {
                db_path,
                sessions_root,
                active_profiles: Mutex::new(HashMap::new()),
                app_sleep_seconds: Mutex::new(HashMap::new()),
                active_webview: Mutex::new(None),
                hibernate_tokens: Mutex::new(HashMap::new()),
                safe_mode: Mutex::new(false),
                notification_rate_limit: Mutex::new(HashMap::new()),
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
                    let (sidebar_w, header_h, footer_h) = webview::layout_offsets(logical_width);

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
            commands::open_app,
            commands::forward_notification,
            commands::request_new_tab,
            commands::close_webview,
            commands::hide_all_webviews,
            commands::get_notifications,
            commands::clear_notifications,
            commands::get_resource_usage,
            commands::set_safe_mode,
            commands::report_webview_error,
            commands::delete_app_session,
            commands::get_encryption_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vessel");
}

#[cfg(test)]
mod tests {
    // Tests can be moved to their respective modules later if desired.
    // For now, keeping them simple or moving to main_test.rs if it exists.
}
