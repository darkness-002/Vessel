fn main() {
    let b = tauri_plugin_stronghold::Builder::new(|password| {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().to_vec()
    }).build();
}
