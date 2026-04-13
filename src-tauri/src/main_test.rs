use tauri::{AppHandle, Manager, webview::WebviewBuilder, WebviewUrl};
fn test(handle: AppHandle) {
    let builder = WebviewBuilder::new("test", WebviewUrl::App(Default::default()));
    // Let's see if this method exists
    let _b2 = builder.data_directory(std::path::PathBuf::from("test"));
}
