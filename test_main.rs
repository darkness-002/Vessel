use tauri::{tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent}, Manager, WindowEvent};
fn test() {
    let _builder = tauri::Builder::default()
        .setup(|app| {
            let main_win = app.get_webview_window("main").unwrap();
            let main_win_clone = main_win.clone();
            
            main_win.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = main_win_clone.hide();
                }
            });

            let icon = app.default_window_icon().unwrap().clone();
            TrayIconBuilder::new()
                .icon(icon)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app).unwrap();
            Ok(())
        });
}
