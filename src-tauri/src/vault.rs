use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rand::RngExt;
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};
use zeroize::{Zeroize, Zeroizing};

pub const VAULT_MAGIC_V1: &[u8; 10] = b"VSSLVAULT1";
pub const VAULT_MAGIC_V2: &[u8; 10] = b"VSSLVAULT2";
pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
pub const HEADER_LEN_V1: usize = 10 + SALT_LEN + NONCE_LEN; // 38 bytes
pub const HEADER_LEN_V2: usize = 10 + 1 + SALT_LEN + NONCE_LEN; // 39 bytes
pub const KEYRING_SERVICE: &str = "vessel_vault";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VaultMode {
    Keyring = 0,
    Passphrase = 1,
}

impl TryFrom<u8> for VaultMode {
    type Error = VaultError;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        match val {
            0 => Ok(VaultMode::Keyring),
            1 => Ok(VaultMode::Passphrase),
            other => Err(VaultError::Format(format!(
                "Unknown vault mode byte: {other}"
            ))),
        }
    }
}

#[derive(Debug)]
pub enum VaultError {
    Io(std::io::Error),
    Crypto(String),
    Keyring(String),
    Format(String),
    InvalidPassphrase,
    VerificationFailed(String),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::Io(e) => write!(f, "Vault IO error: {e}"),
            VaultError::Crypto(e) => write!(f, "Cryptographic error: {e}"),
            VaultError::Keyring(e) => write!(f, "Keyring error: {e}"),
            VaultError::Format(e) => write!(f, "Vault format error: {e}"),
            VaultError::InvalidPassphrase => {
                write!(f, "Invalid vault passphrase or corrupted payload")
            }
            VaultError::VerificationFailed(e) => write!(f, "Vault verification error: {e}"),
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

pub trait KeyringProvider: Send + Sync {
    fn get_password(&self, service: &str, account: &str) -> Result<String, VaultError>;
    fn set_password(&self, service: &str, account: &str, password: &str) -> Result<(), VaultError>;
    fn delete_credential(&self, service: &str, account: &str) -> Result<(), VaultError>;
}

pub struct OsKeyring;

impl KeyringProvider for OsKeyring {
    fn get_password(&self, service: &str, account: &str) -> Result<String, VaultError> {
        let entry = keyring::Entry::new(service, account)
            .map_err(|e| VaultError::Keyring(format!("Keyring initialization failed: {e}")))?;
        entry
            .get_password()
            .map_err(|e| VaultError::Keyring(format!("Keyring credential retrieval failed: {e}")))
    }

    fn set_password(&self, service: &str, account: &str, password: &str) -> Result<(), VaultError> {
        let entry = keyring::Entry::new(service, account)
            .map_err(|e| VaultError::Keyring(format!("Keyring initialization failed: {e}")))?;
        entry
            .set_password(password)
            .map_err(|e| VaultError::Keyring(format!("Keyring credential storage failed: {e}")))
    }

    fn delete_credential(&self, service: &str, account: &str) -> Result<(), VaultError> {
        let entry = keyring::Entry::new(service, account)
            .map_err(|e| VaultError::Keyring(format!("Keyring initialization failed: {e}")))?;
        let _ = entry.delete_credential();
        Ok(())
    }
}

pub fn load_key(container_id: &str) -> Result<Zeroizing<[u8; 32]>, VaultError> {
    load_key_with_provider(container_id, &OsKeyring)
}

pub fn load_key_with_provider<K: KeyringProvider>(
    container_id: &str,
    provider: &K,
) -> Result<Zeroizing<[u8; 32]>, VaultError> {
    let account = format!("vault_{}", container_id);
    let hex_key = provider.get_password(KEYRING_SERVICE, &account)?;
    let bytes = hex_to_bytes(&hex_key)
        .map_err(|e| VaultError::Format(format!("Corrupted hex key in keyring: {e}")))?;
    if bytes.len() != 32 {
        return Err(VaultError::Format(
            "Retrieved keyring key length is not 32 bytes".to_string(),
        ));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(Zeroizing::new(key))
}

pub fn create_and_store_key(container_id: &str) -> Result<Zeroizing<[u8; 32]>, VaultError> {
    create_and_store_key_with_provider(container_id, &OsKeyring)
}

pub fn create_and_store_key_with_provider<K: KeyringProvider>(
    container_id: &str,
    provider: &K,
) -> Result<Zeroizing<[u8; 32]>, VaultError> {
    let account = format!("vault_{}", container_id);
    let mut raw_key = [0u8; 32];
    rand::rng().fill(&mut raw_key);
    let key = Zeroizing::new(raw_key);
    let hex_key = bytes_to_hex(key.as_ref());

    provider.set_password(KEYRING_SERVICE, &account, &hex_key)?;

    let read_back = provider.get_password(KEYRING_SERVICE, &account)?;
    if read_back != hex_key {
        return Err(VaultError::Keyring(
            "Keyring durable storage verification failed: read-back key mismatch".to_string(),
        ));
    }

    Ok(key)
}

pub fn load_or_create_key(container_id: &str) -> Result<Zeroizing<[u8; 32]>, VaultError> {
    load_or_create_key_with_provider(container_id, &OsKeyring)
}

pub fn load_or_create_key_with_provider<K: KeyringProvider>(
    container_id: &str,
    provider: &K,
) -> Result<Zeroizing<[u8; 32]>, VaultError> {
    if let Ok(existing_key) = load_key_with_provider(container_id, provider) {
        return Ok(existing_key);
    }
    create_and_store_key_with_provider(container_id, provider)
}

pub fn delete_vault_key(container_id: &str) -> Result<(), VaultError> {
    delete_vault_key_with_provider(container_id, &OsKeyring)
}

pub fn delete_vault_key_with_provider<K: KeyringProvider>(
    container_id: &str,
    provider: &K,
) -> Result<(), VaultError> {
    let account = format!("vault_{}", container_id);
    provider.delete_credential(KEYRING_SERVICE, &account)
}

pub fn derive_key_from_passphrase(
    passphrase: &str,
    salt: &[u8; SALT_LEN],
) -> Result<Zeroizing<[u8; 32]>, VaultError> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| VaultError::Crypto(format!("Argon2 key derivation failed: {e}")))?;
    Ok(Zeroizing::new(key))
}

pub fn read_vault_mode(vault_path: &Path) -> Result<VaultMode, VaultError> {
    let mut file = std::fs::File::open(vault_path)?;
    let mut magic_and_mode = [0u8; 11];
    let n = file.read(&mut magic_and_mode)?;

    if n >= 10 && &magic_and_mode[..10] == VAULT_MAGIC_V1 {
        return Ok(VaultMode::Keyring);
    }

    if n >= 11 && &magic_and_mode[..10] == VAULT_MAGIC_V2 {
        return VaultMode::try_from(magic_and_mode[10]);
    }

    Err(VaultError::Format(
        "Invalid vault file magic signature or unsupported version".to_string(),
    ))
}

pub fn read_vault_salt(vault_path: &Path) -> Result<[u8; SALT_LEN], VaultError> {
    let mut file = std::fs::File::open(vault_path)?;
    let mut header = [0u8; HEADER_LEN_V2];
    let n = file.read(&mut header)?;

    if n >= HEADER_LEN_V2 && &header[..10] == VAULT_MAGIC_V2 {
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&header[11..11 + SALT_LEN]);
        return Ok(salt);
    }

    if n >= HEADER_LEN_V1 && &header[..10] == VAULT_MAGIC_V1 {
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&header[10..10 + SALT_LEN]);
        return Ok(salt);
    }

    Err(VaultError::Format(
        "Invalid vault file header length or magic signature".to_string(),
    ))
}

pub fn secure_wipe_file(path: &Path) -> std::io::Result<()> {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.is_file() {
            let len = meta.len() as usize;
            if len > 0 {
                if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(path) {
                    let chunk_size = std::cmp::min(len, 64 * 1024);
                    let mut zeroes = vec![0u8; chunk_size];
                    let mut written = 0;
                    while written < len {
                        let to_write = std::cmp::min(zeroes.len(), len - written);
                        let _ = file.write_all(&zeroes[..to_write]);
                        written += to_write;
                    }
                    let _ = file.flush();
                    let _ = file.sync_all();
                    zeroes.zeroize();
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

fn compute_buffer_hash(data: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

pub fn pack_and_encrypt_vault(
    container_id: &str,
    profile_dir: &Path,
    vault_dest: &Path,
    key: &[u8; 32],
) -> Result<(), VaultError> {
    pack_and_encrypt_vault_with_mode(
        container_id,
        profile_dir,
        vault_dest,
        VaultMode::Keyring,
        key,
        None,
    )
}

pub fn pack_and_encrypt_vault_with_mode(
    _container_id: &str,
    profile_dir: &Path,
    vault_dest: &Path,
    mode: VaultMode,
    key: &[u8; 32],
    salt: Option<[u8; SALT_LEN]>,
) -> Result<(), VaultError> {
    // 1. In-memory tar-gzip compression
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

    let original_hash = compute_buffer_hash(&tar_buffer);

    // 2. Cryptographic Salt & Nonce Generation
    let salt = salt.unwrap_or_else(|| {
        let mut s = [0u8; SALT_LEN];
        rand::rng().fill(&mut s);
        s
    });
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill(&mut nonce_bytes);

    let nonce = Nonce::try_from(nonce_bytes.as_slice())
        .map_err(|e| VaultError::Crypto(format!("Invalid nonce: {e}")))?;

    // 3. Authenticated Symmetric Encryption (AES-256-GCM)
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| VaultError::Crypto(format!("Invalid AES key: {e}")))?;

    let ciphertext = cipher
        .encrypt(&nonce, tar_buffer.as_ref())
        .map_err(|e| VaultError::Crypto(format!("Vault encryption failed: {e}")))?;

    // 4. Atomic Vault File Write with fsync
    if let Some(parent) = vault_dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let unique_suffix = rand::rng().random::<u64>();
    let temp_dest = vault_dest.with_extension(format!("tmp.{}", unique_suffix));

    {
        let mut out = std::fs::File::create(&temp_dest)?;
        out.write_all(VAULT_MAGIC_V2)?;
        out.write_all(&[mode as u8])?;
        out.write_all(&salt)?;
        out.write_all(&nonce_bytes)?;
        out.write_all(&ciphertext)?;
        out.flush()?;
        out.sync_all()?;
    }

    std::fs::rename(&temp_dest, vault_dest)?;

    // 5. Verification step: Decrypt written vault in memory and verify hash matches before wiping plaintext
    let written_bytes = std::fs::read(vault_dest)?;
    if written_bytes.len() < HEADER_LEN_V2 {
        return Err(VaultError::VerificationFailed(
            "Written vault is truncated".to_string(),
        ));
    }

    let written_nonce_bytes = &written_bytes[11 + SALT_LEN..HEADER_LEN_V2];
    let written_ciphertext = &written_bytes[HEADER_LEN_V2..];
    let written_nonce = Nonce::try_from(written_nonce_bytes)
        .map_err(|e| VaultError::VerificationFailed(format!("Invalid written nonce: {e}")))?;

    let mut decrypted_verify = cipher
        .decrypt(&written_nonce, written_ciphertext)
        .map_err(|e| {
            VaultError::VerificationFailed(format!(
                "Decryption verification of written vault failed: {e}"
            ))
        })?;

    let verified_hash = compute_buffer_hash(&decrypted_verify);
    if verified_hash != original_hash || decrypted_verify != tar_buffer {
        decrypted_verify.zeroize();
        return Err(VaultError::VerificationFailed(
            "Checksum mismatch: decrypted verification archive differs from original payload"
                .to_string(),
        ));
    }
    decrypted_verify.zeroize();

    // 6. Secure zeroing and removal of plaintext profile only after verified decryption
    let _ = secure_wipe_dir(profile_dir);

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

    let (nonce_bytes, ciphertext) =
        if vault_data.len() >= HEADER_LEN_V2 && &vault_data[..10] == VAULT_MAGIC_V2 {
            let nonce = &vault_data[11 + SALT_LEN..HEADER_LEN_V2];
            let cipher = &vault_data[HEADER_LEN_V2..];
            (nonce, cipher)
        } else if vault_data.len() >= HEADER_LEN_V1 && &vault_data[..10] == VAULT_MAGIC_V1 {
            let nonce = &vault_data[10 + SALT_LEN..HEADER_LEN_V1];
            let cipher = &vault_data[HEADER_LEN_V1..];
            (nonce, cipher)
        } else if vault_data.len() < HEADER_LEN_V1 {
            return Err(VaultError::Format("Corrupted vault: file too small".into()));
        } else {
            return Err(VaultError::Format(
                "Invalid vault file magic signature".into(),
            ));
        };

    let nonce = Nonce::try_from(nonce_bytes)
        .map_err(|e| VaultError::Crypto(format!("Invalid nonce: {e}")))?;

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| VaultError::Crypto(format!("Invalid AES key: {e}")))?;

    let mut decrypted_tar = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|_| VaultError::InvalidPassphrase)?;

    std::fs::create_dir_all(profile_dest)?;

    let decoder = GzDecoder::new(Cursor::new(&decrypted_tar));
    let mut archive = Archive::new(decoder);
    archive.unpack(profile_dest)?;

    let _ = std::fs::remove_file(vault_src);

    decrypted_tar.zeroize();
    vault_data.zeroize();

    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("Invalid hex length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::Mutex;

    pub struct MockKeyring {
        pub store: Mutex<std::collections::HashMap<String, String>>,
        pub should_fail_write: Mutex<bool>,
    }

    impl MockKeyring {
        pub fn new() -> Self {
            Self {
                store: Mutex::new(std::collections::HashMap::new()),
                should_fail_write: Mutex::new(false),
            }
        }
    }

    impl KeyringProvider for MockKeyring {
        fn get_password(&self, service: &str, account: &str) -> Result<String, VaultError> {
            let key = format!("{}:{}", service, account);
            let lock = self.store.lock().unwrap();
            lock.get(&key)
                .cloned()
                .ok_or_else(|| VaultError::Keyring("Mock credential not found".to_string()))
        }

        fn set_password(
            &self,
            service: &str,
            account: &str,
            password: &str,
        ) -> Result<(), VaultError> {
            if *self.should_fail_write.lock().unwrap() {
                return Err(VaultError::Keyring(
                    "Simulated keyring write failure".to_string(),
                ));
            }
            let key = format!("{}:{}", service, account);
            self.store.lock().unwrap().insert(key, password.to_string());
            Ok(())
        }

        fn delete_credential(&self, service: &str, account: &str) -> Result<(), VaultError> {
            let key = format!("{}:{}", service, account);
            self.store.lock().unwrap().remove(&key);
            Ok(())
        }
    }

    fn create_test_profile(profile_dir: &Path) {
        std::fs::create_dir_all(profile_dir).unwrap();
        std::fs::write(
            profile_dir.join("cookies.sqlite"),
            b"SQLITE_TEST_COOKIE_PAYLOAD_12345",
        )
        .unwrap();
        std::fs::write(
            profile_dir.join("preferences.json"),
            b"{\"theme\": \"dark\", \"session\": \"active\"}",
        )
        .unwrap();
    }

    // 1. Round trip in both modes (Keyring mode and Passphrase mode)
    #[test]
    fn test_vault_roundtrip_keyring_mode() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_test_keyring_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let profile_dir = temp_dir.join("profile");
        let vault_file = temp_dir.join("test_keyring.vault");
        let restored_dir = temp_dir.join("restored");

        create_test_profile(&profile_dir);

        let mock_keyring = MockKeyring::new();
        let key = create_and_store_key_with_provider("test_app", &mock_keyring).unwrap();

        pack_and_encrypt_vault_with_mode(
            "test_app",
            &profile_dir,
            &vault_file,
            VaultMode::Keyring,
            &key,
            None,
        )
        .expect("Packing and encryption in Keyring mode should succeed");

        assert!(vault_file.exists());
        assert!(
            !profile_dir.exists(),
            "Plaintext profile should be wiped after verified seal"
        );

        let loaded_mode = read_vault_mode(&vault_file).unwrap();
        assert_eq!(loaded_mode, VaultMode::Keyring);

        let retrieved_key = load_key_with_provider("test_app", &mock_keyring).unwrap();
        assert_eq!(*key, *retrieved_key);

        decrypt_and_unpack_vault("test_app", &vault_file, &restored_dir, &retrieved_key)
            .expect("Decryption should succeed");

        assert!(!vault_file.exists());
        assert!(restored_dir.join("cookies.sqlite").exists());
        assert_eq!(
            std::fs::read(restored_dir.join("cookies.sqlite")).unwrap(),
            b"SQLITE_TEST_COOKIE_PAYLOAD_12345"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_vault_roundtrip_passphrase_mode() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_test_passphrase_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let profile_dir = temp_dir.join("profile");
        let vault_file = temp_dir.join("test_pass.vault");
        let restored_dir = temp_dir.join("restored");

        create_test_profile(&profile_dir);

        let passphrase = "correct-horse-battery-staple";
        let salt = [42u8; SALT_LEN];
        let key = derive_key_from_passphrase(passphrase, &salt).unwrap();

        pack_and_encrypt_vault_with_mode(
            "test_pass",
            &profile_dir,
            &vault_file,
            VaultMode::Passphrase,
            &key,
            Some(salt),
        )
        .expect("Packing and encryption in Passphrase mode should succeed");

        assert!(vault_file.exists());
        assert!(!profile_dir.exists());

        let mode = read_vault_mode(&vault_file).unwrap();
        assert_eq!(mode, VaultMode::Passphrase);

        let read_salt = read_vault_salt(&vault_file).unwrap();
        let derived_unlock_key = derive_key_from_passphrase(passphrase, &read_salt).unwrap();

        decrypt_and_unpack_vault("test_pass", &vault_file, &restored_dir, &derived_unlock_key)
            .expect("Decryption with matching passphrase should succeed");

        assert!(restored_dir.join("cookies.sqlite").exists());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // 2. Wrong passphrase
    #[test]
    fn test_vault_wrong_passphrase_fails() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_wrong_pass_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let profile_dir = temp_dir.join("profile");
        let vault_file = temp_dir.join("test_wrong_pass.vault");
        let restored_dir = temp_dir.join("restored");

        create_test_profile(&profile_dir);

        let passphrase = "correct_secret_passphrase";
        let salt = [77u8; SALT_LEN];
        let key = derive_key_from_passphrase(passphrase, &salt).unwrap();

        pack_and_encrypt_vault_with_mode(
            "test_wrong",
            &profile_dir,
            &vault_file,
            VaultMode::Passphrase,
            &key,
            Some(salt),
        )
        .unwrap();

        let read_salt = read_vault_salt(&vault_file).unwrap();
        let wrong_key = derive_key_from_passphrase("wrong_passphrase", &read_salt).unwrap();

        let result = decrypt_and_unpack_vault("test_wrong", &vault_file, &restored_dir, &wrong_key);
        assert!(result.is_err());
        assert!(
            vault_file.exists(),
            "Vault file must not be deleted on failed decryption"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // 3. Wrong key
    #[test]
    fn test_vault_wrong_key_fails() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_wrong_key_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let profile_dir = temp_dir.join("profile");
        let vault_file = temp_dir.join("test_wrong_key.vault");
        let restored_dir = temp_dir.join("restored");

        create_test_profile(&profile_dir);

        let key = [1u8; 32];
        let wrong_key = [2u8; 32];

        pack_and_encrypt_vault_with_mode(
            "test_app",
            &profile_dir,
            &vault_file,
            VaultMode::Keyring,
            &key,
            None,
        )
        .unwrap();

        let res = decrypt_and_unpack_vault("test_app", &vault_file, &restored_dir, &wrong_key);
        assert!(res.is_err());
        assert!(vault_file.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // 4. Tampered ciphertext (flip one byte)
    #[test]
    fn test_vault_tampered_ciphertext_fails() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_tamper_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let profile_dir = temp_dir.join("profile");
        let vault_file = temp_dir.join("test_tamper.vault");
        let restored_dir = temp_dir.join("restored");

        create_test_profile(&profile_dir);
        let key = [5u8; 32];

        pack_and_encrypt_vault_with_mode(
            "test_tamper",
            &profile_dir,
            &vault_file,
            VaultMode::Keyring,
            &key,
            None,
        )
        .unwrap();

        // Tamper with one byte in the ciphertext payload
        let mut bytes = std::fs::read(&vault_file).unwrap();
        let last_idx = bytes.len() - 1;
        bytes[last_idx] ^= 0xFF;
        std::fs::write(&vault_file, bytes).unwrap();

        let res = decrypt_and_unpack_vault("test_tamper", &vault_file, &restored_dir, &key);
        assert!(
            res.is_err(),
            "Decryption of tampered ciphertext must fail authentication"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // 5. Truncated file
    #[test]
    fn test_vault_truncated_file_fails() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_trunc_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let profile_dir = temp_dir.join("profile");
        let vault_file = temp_dir.join("test_trunc.vault");
        let restored_dir = temp_dir.join("restored");

        create_test_profile(&profile_dir);
        let key = [7u8; 32];

        pack_and_encrypt_vault_with_mode(
            "test_trunc",
            &profile_dir,
            &vault_file,
            VaultMode::Keyring,
            &key,
            None,
        )
        .unwrap();

        // Truncate file to less than header length
        std::fs::write(&vault_file, b"VSSLVAULT2").unwrap();

        let res = decrypt_and_unpack_vault("test_trunc", &vault_file, &restored_dir, &key);
        assert!(res.is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // 6. Bad magic signature
    #[test]
    fn test_vault_bad_magic_fails() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_badmagic_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let vault_file = temp_dir.join("badmagic.vault");
        let restored_dir = temp_dir.join("restored");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut data = vec![0u8; 60];
        data[..10].copy_from_slice(b"BADMAGIC99");
        std::fs::write(&vault_file, data).unwrap();

        let res = decrypt_and_unpack_vault("test", &vault_file, &restored_dir, &[0u8; 32]);
        assert!(res.is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // 7. Keyring write failure preserves plaintext profile
    #[test]
    fn test_keyring_write_failure_preserves_plaintext() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_keyring_fail_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let profile_dir = temp_dir.join("profile");
        create_test_profile(&profile_dir);

        let mock_keyring = MockKeyring::new();
        *mock_keyring.should_fail_write.lock().unwrap() = true; // Inject keyring write failure

        let res = create_and_store_key_with_provider("failing_app", &mock_keyring);
        assert!(
            res.is_err(),
            "create_and_store_key must return error when keyring write fails"
        );

        // Profile directory must remain untouched and intact
        assert!(
            profile_dir.exists(),
            "Plaintext profile must be preserved if key storage fails"
        );
        assert!(profile_dir.join("cookies.sqlite").exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // 8. Leftover .tmp file from interrupted seal does not break next seal/unlock
    #[test]
    fn test_leftover_tmp_file_does_not_break_seal_or_unlock() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_leftover_tmp_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let profile_dir = temp_dir.join("profile");
        let vault_file = temp_dir.join("test_tmp.vault");
        let stale_tmp = temp_dir.join("test_tmp.tmp");
        let restored_dir = temp_dir.join("restored");

        create_test_profile(&profile_dir);
        let key = [9u8; 32];

        // Simulate a leftover stale .tmp file from a killed process
        std::fs::write(
            &stale_tmp,
            b"CORRUPTED_LEFTOVER_TEMP_DATA_FROM_PREVIOUS_CRASH",
        )
        .unwrap();

        // Packing must succeed cleanly despite stale tmp
        pack_and_encrypt_vault_with_mode(
            "test_tmp",
            &profile_dir,
            &vault_file,
            VaultMode::Keyring,
            &key,
            None,
        )
        .expect("Packing should succeed even with leftover .tmp file");

        assert!(vault_file.exists());

        // Decryption should succeed
        decrypt_and_unpack_vault("test_tmp", &vault_file, &restored_dir, &key)
            .expect("Unlocking must succeed");
        assert!(restored_dir.join("cookies.sqlite").exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // 9. V1 vault format is still readable
    #[test]
    fn test_v1_vault_still_readable() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_v1_compat_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let profile_dir = temp_dir.join("profile");
        let vault_file = temp_dir.join("v1.vault");
        let restored_dir = temp_dir.join("restored");

        create_test_profile(&profile_dir);
        let key = [3u8; 32];

        // Manually build a valid V1 vault file
        let mut tar_buffer = Vec::new();
        {
            let gz = GzEncoder::new(&mut tar_buffer, Compression::default());
            let mut builder = Builder::new(gz);
            for entry in std::fs::read_dir(&profile_dir).unwrap() {
                let entry = entry.unwrap();
                let mut f = std::fs::File::open(entry.path()).unwrap();
                builder.append_file(entry.file_name(), &mut f).unwrap();
            }
            let gz = builder.into_inner().unwrap();
            gz.finish().unwrap();
        }

        let salt = [11u8; SALT_LEN];
        let nonce_bytes = [22u8; NONCE_LEN];
        let nonce = Nonce::try_from(nonce_bytes.as_slice()).unwrap();
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let ciphertext = cipher.encrypt(&nonce, tar_buffer.as_ref()).unwrap();

        // Write V1 header (10 magic + 16 salt + 12 nonce = 38 bytes)
        let mut v1_file = std::fs::File::create(&vault_file).unwrap();
        v1_file.write_all(VAULT_MAGIC_V1).unwrap();
        v1_file.write_all(&salt).unwrap();
        v1_file.write_all(&nonce_bytes).unwrap();
        v1_file.write_all(&ciphertext).unwrap();
        v1_file.flush().unwrap();
        drop(v1_file);

        // Verify V1 format detection
        let mode = read_vault_mode(&vault_file).unwrap();
        assert_eq!(mode, VaultMode::Keyring);

        // Verify V1 decryption
        decrypt_and_unpack_vault("v1_app", &vault_file, &restored_dir, &key).unwrap();
        assert!(restored_dir.join("cookies.sqlite").exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // 10. Two seals of the same profile produce different nonces
    #[test]
    fn test_two_seals_produce_different_nonces() {
        let temp_dir = std::env::temp_dir().join(format!(
            "vessel_vault_nonces_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let profile1 = temp_dir.join("p1");
        let profile2 = temp_dir.join("p2");
        let vault1 = temp_dir.join("v1.vault");
        let vault2 = temp_dir.join("v2.vault");

        create_test_profile(&profile1);
        create_test_profile(&profile2);

        let key = [4u8; 32];

        pack_and_encrypt_vault_with_mode("t1", &profile1, &vault1, VaultMode::Keyring, &key, None)
            .unwrap();
        pack_and_encrypt_vault_with_mode("t2", &profile2, &vault2, VaultMode::Keyring, &key, None)
            .unwrap();

        let v1_data = std::fs::read(&vault1).unwrap();
        let v2_data = std::fs::read(&vault2).unwrap();

        // Nonce is at 11 + SALT_LEN .. HEADER_LEN_V2
        let nonce1 = &v1_data[11 + SALT_LEN..HEADER_LEN_V2];
        let nonce2 = &v2_data[11 + SALT_LEN..HEADER_LEN_V2];

        assert_ne!(
            nonce1, nonce2,
            "Two distinct seal operations must never reuse nonces"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
