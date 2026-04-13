use tauri::webview::WebviewBuilder;
fn test() {
    let builder = WebviewBuilder::new("test", tauri::WebviewUrl::App(Default::default()));
    let builder = builder.data_directory(std::path::PathBuf::from("test"));
}
