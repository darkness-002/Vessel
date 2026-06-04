fn main() {
    let _b = tauri_plugin_stronghold::Builder::new(|password| password.as_bytes().to_vec());
}
