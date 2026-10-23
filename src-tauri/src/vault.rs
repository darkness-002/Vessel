use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rand::RngExt;
use tar::{Archive, Builder};
use zeroize::Zeroize;

pub const VAULT_MAGIC: &[u8; 10] = b"VSSLVAULT1";
pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
pub const HEADER_LEN: usize = 10 + SALT_LEN + NONCE_LEN; // 38 bytes
pub const KEYRING_SERVICE: &str = "vessel_vault";

#[derive(Debug)]
pub enum VaultError {
    Io(std::io::Error),
    Crypto(String),
    Keyring(String),
    Format(String),
    InvalidPassphrase,
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::Io(e) => write!(f, "Vault IO error: {e}"),
            VaultError::Crypto(e) => write!(f, "Cryptographic error: {e}"),
            VaultError::Keyring(e) => write!(f, "Keyring error: {e}"),
            VaultError::Format(e) => write!(f, "Vault format error: {e}"),
            VaultError::InvalidPassphrase => write!(f, "Invalid vault passphrase or corrupted payload"),
        }
    }
}

impl std::error::Error for VaultError {}

impl From<std::io::Error> for VaultError {
    fn from(err: std::io::Error) -> Self {
        VaultError::Io(err)
    }
}

pub fn get_vault_dir() -> PathBuf {
    let dir = crate::profile::get_vessel_root().join("vaults");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn get_vault_path(container_id: &str) -> PathBuf {
    get_vault_dir().join(format!("{}.vault", container_id))
}

pub fn is_vault_locked(container_id: &str) -> bool {
    let vault_file = get_vault_path(container_id);
    let profile_dir = crate::profile::get_profile_dir(container_id);
    vault_file.exists() && (!profile_dir.exists() || is_dir_empty(&profile_dir))
}

fn is_dir_empty(path: &Path) -> bool {
    if !path.exists() {
        return true;
    }
    std::fs::read_dir(path)
        .map(|mut i| i.next().is_none())
        .unwrap_or(true)
}

pub fn derive_key_from_passphrase(passphrase: &str, salt: &[u8; SALT_LEN]) -> Result<[u8; 32], VaultError> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| VaultError::Crypto(format!("Argon2 key derivation failed: {e}")))?;
    Ok(key)
}

pub fn get_or_create_vault_key(container_id: &str) -> Result<[u8; 32], VaultError> {
    let account = format!("vault_{}", container_id);
    let entry = keyring::Entry::new(KEYRING_SERVICE, &account)
        .map_err(|e| VaultError::Keyring(format!("Keyring initialization failed: {e}")))?;

    if let Ok(stored_hex) = entry.get_password() {
        if let Ok(decoded) = hex_to_bytes(&stored_hex) {
            if decoded.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&decoded);
                return Ok(key);
            }
        }
    }

    // Generate random 256-bit key
    let mut new_key = [0u8; 32];
    rand::rng().fill(&mut new_key);
    let hex_key = bytes_to_hex(&new_key);

    let _ = entry.set_password(&hex_key);
    Ok(new_key)
}

pub fn store_vault_key(container_id: &str, key: &[u8; 32]) -> Result<(), VaultError> {
    let account = format!("vault_{}", container_id);
    let entry = keyring::Entry::new(KEYRING_SERVICE, &account)
        .map_err(|e| VaultError::Keyring(format!("Keyring init failed: {e}")))?;
    let hex_key = bytes_to_hex(key);
    entry
        .set_password(&hex_key)
        .map_err(|e| VaultError::Keyring(format!("Failed to store vault key in keyring: {e}")))
}

pub fn delete_vault_key(container_id: &str) -> Result<(), VaultError> {
    let account = format!("vault_{}", container_id);
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &account) {
        let _ = entry.delete_credential();
    }
    Ok(())
}

pub fn read_vault_salt(vault_path: &Path) -> Result<[u8; SALT_LEN], VaultError> {
    let mut file = std::fs::File::open(vault_path)?;
    let mut header = [0u8; HEADER_LEN];
    file.read_exact(&mut header)?;

    if &header[..10] != VAULT_MAGIC {
        return Err(VaultError::Format("Invalid vault file magic signature".to_string()));
    }

    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&header[10..10 + SALT_LEN]);
    Ok(salt)
}

pub fn secure_wipe_file(path: &Path) -> std::io::Result<()> {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.is_file() {
            let len = meta.len() as usize;
            if len > 0 {
                if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(path) {
                    let chunk_size = std::cmp::min(len, 64 * 1024);
                    let zeroes = vec![0u8; chunk_size];
                    let mut written = 0;
                    while written < len {
                        let to_write = std::cmp::min(zeroes.len(), len - written);
                        let _ = file.write_all(&zeroes[..to_write]);
                        written += to_write;
                    }
                    let _ = file.flush();
                }
            }
        }
    }
    std::fs::remove_file(path)
}

pub fn secure_wipe_dir(dir: &Path) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let _ = secure_wipe_dir(&path);
        } else {
            let _ = secure_wipe_file(&path);
        }
    }
    std::fs::remove_dir_all(dir)
}

pub fn pack_and_encrypt_vault(
    _container_id: &str,
    profile_dir: &Path,
    vault_dest: &Path,
    key: &[u8; 32],
) -> Result<(), VaultError> {
    // 1. In-memory tar compression
    let mut tar_buffer = Vec::new();
    {
        let gz = GzEncoder::new(&mut tar_buffer, Compression::default());
        let mut builder = Builder::new(gz);

        if profile_dir.exists() {
            for entry in std::fs::read_dir(profile_dir)? {
                let entry = entry?;
                let path = entry.path();
                let file_name = entry.file_name();
                if path.is_dir() {
                    builder.append_dir_all(&file_name, &path)?;
                } else {
                    let mut f = std::fs::File::open(&path)?;
                    builder.append_file(&file_name, &mut f)?;
                }
            }
        }

        let gz = builder.into_inner()?;
        gz.finish()?;
    }

    // 2. Cryptographic Salt & Nonce Generation
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill(&mut salt);
    rand::rng().fill(&mut nonce_bytes);

    let nonce = Nonce::try_from(nonce_bytes.as_slice())
        .map_err(|e| VaultError::Crypto(format!("Invalid nonce: {e}")))?;

    // 3. Authenticated Symmetric Encryption (AES-256-GCM)
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| VaultError::Crypto(format!("Invalid AES key: {e}")))?;

    let ciphertext = cipher
        .encrypt(&nonce, tar_buffer.as_ref())
        .map_err(|e| VaultError::Crypto(format!("Vault encryption failed: {e}")))?;

    // 4. Atomic Vault File Write
    if let Some(parent) = vault_dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let temp_dest = vault_dest.with_extension("tmp");
    {
        let mut out = std::fs::File::create(&temp_dest)?;
        out.write_all(VAULT_MAGIC)?;
        out.write_all(&salt)?;
        out.write_all(&nonce_bytes)?;
        out.write_all(&ciphertext)?;
        out.flush()?;
    }
    std::fs::rename(temp_dest, vault_dest)?;

    // 5. Secure Zeroing and Removal of Unencrypted Data
    let _ = secure_wipe_dir(profile_dir);

    // Zeroize sensitive in-memory buffers
    let mut key_clean = *key;
    key_clean.zeroize();
    tar_buffer.zeroize();

    Ok(())
}

pub fn decrypt_and_unpack_vault(
    _container_id: &str,
    vault_src: &Path,
    profile_dest: &Path,
    key: &[u8; 32],
) -> Result<(), VaultError> {
    if !vault_src.exists() {
        return Err(VaultError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Vault file {:?} does not exist", vault_src),
        )));
    }

    let mut vault_data = std::fs::read(vault_src)?;
    if vault_data.len() < HEADER_LEN {
        return Err(VaultError::Format("Corrupted vault: header too small".into()));
    }

    // Verify magic header
    if &vault_data[..10] != VAULT_MAGIC {
        return Err(VaultError::Format("Invalid vault file magic signature".into()));
    }

    let nonce_bytes = &vault_data[10 + SALT_LEN..HEADER_LEN];
    let ciphertext = &vault_data[HEADER_LEN..];

    let nonce = Nonce::try_from(nonce_bytes)
        .map_err(|e| VaultError::Crypto(format!("Invalid nonce: {e}")))?;

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| VaultError::Crypto(format!("Invalid AES key: {e}")))?;

    let mut decrypted_tar = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|_| VaultError::InvalidPassphrase)?;

    // Prepare profile destination
    std::fs::create_dir_all(profile_dest)?;

    // Decompress and unpack archive
    let decoder = GzDecoder::new(Cursor::new(&decrypted_tar));
    let mut archive = Archive::new(decoder);
    archive.unpack(profile_dest)?;

    // Clean up vault file after successful extraction
    let _ = std::fs::remove_file(vault_src);

    // Zeroize plaintext archive
    decrypted_tar.zeroize();
    vault_data.zeroize();

    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Invalid hex length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_pack_and_unpack() {
        let temp_dir = std::env::temp_dir().join(format!("vessel_vault_test_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let profile_dir = temp_dir.join("profile");
        let vault_file = temp_dir.join("test.vault");
        let restored_dir = temp_dir.join("restored");

        std::fs::create_dir_all(&profile_dir).unwrap();
        std::fs::write(profile_dir.join("cookies.sqlite"), b"SESSION_TOKEN_12345").unwrap();
        std::fs::write(profile_dir.join("localstorage.json"), b"{\"auth\":true}").unwrap();

        let mut key = [0u8; 32];
        rand::rng().fill(&mut key);

        // Pack & Encrypt
        pack_and_encrypt_vault("test-app", &profile_dir, &vault_file, &key)
            .expect("Packing and encryption should succeed");

        assert!(vault_file.exists());
        assert!(!profile_dir.exists(), "Original profile dir should be securely deleted");

        // Decrypt & Unpack
        decrypt_and_unpack_vault("test-app", &vault_file, &restored_dir, &key)
            .expect("Decryption and unpacking should succeed");

        assert!(!vault_file.exists(), "Vault file should be cleaned up on extraction");
        assert!(restored_dir.join("cookies.sqlite").exists());
        assert_eq!(
            std::fs::read(restored_dir.join("cookies.sqlite")).unwrap(),
            b"SESSION_TOKEN_12345"
        );
        assert_eq!(
            std::fs::read(restored_dir.join("localstorage.json")).unwrap(),
            b"{\"auth\":true}"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_argon2_key_derivation() {
        let salt = [42u8; SALT_LEN];
        let key1 = derive_key_from_passphrase("correct_horse_battery_staple", &salt).unwrap();
        let key2 = derive_key_from_passphrase("correct_horse_battery_staple", &salt).unwrap();
        let key3 = derive_key_from_passphrase("wrong_passphrase", &salt).unwrap();

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_vault_invalid_key_fails() {
        let temp_dir = std::env::temp_dir().join(format!("vessel_vault_fail_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let profile_dir = temp_dir.join("profile");
        let vault_file = temp_dir.join("test.vault");
        let restored_dir = temp_dir.join("restored");

        std::fs::create_dir_all(&profile_dir).unwrap();
        std::fs::write(profile_dir.join("test.txt"), b"DATA").unwrap();

        let key = [1u8; 32];
        let bad_key = [2u8; 32];

        pack_and_encrypt_vault("test-app", &profile_dir, &vault_file, &key).unwrap();

        let res = decrypt_and_unpack_vault("test-app", &vault_file, &restored_dir, &bad_key);
        assert!(res.is_err(), "Decryption with wrong key must fail");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
