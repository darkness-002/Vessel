# Security & Threat Model

Vessel implements a defense-in-depth isolation and encryption model designed to keep web applications, cookies, and authentication tokens strictly separated from one another and protected at rest.

---

## 1. Threat Model & Security Boundaries

### What the Encrypted Profile Vault Protects Against:
* **Offline Disk Theft & Cold Extraction**: When a container is locked or closed, profile directories (containing session cookies, IndexedDB records, and local storage) are compressed into an authenticated `AES-256-GCM` archive. An attacker with physical access to the powered-off disk or disk image cannot inspect sensitive session cookies without the cryptographic key.
* **Unencrypted File Backup Leaks**: Automated file-level backup solutions capture only the encrypted `.vault` files, not raw session databases.
* **Multi-App Profile Partitioning**: Browser sessions and cookie jars are physically separated on disk. An untrusted script in one container cannot read or modify tokens belonging to another container.

### What the Vault Does NOT Protect Against:
* **Malware Running in the User Context**: If untrusted code or malware runs on the host system under the same OS user account, it can query the native OS credential store (Keyring/DPAPI) or inspect active application memory.
* **Active, Unlocked Container Sessions**: When a container is running, its profile directory is temporarily decrypted and unpacked on the filesystem so the native WebView2 runtime can execute browser operations.
* **Solid-State Drive (SSD) Wear-Leveling Remnants**: Modern solid-state drives utilize Flash Translation Layers (FTL) that relocate physical blocks during write operations. While Vessel performs multi-pass zero-filling (`secure_wipe`) before unlinking files, software overwriting is strictly **best-effort** and cannot guarantee physical block erasure on SSDs. Full-Disk Encryption (e.g., BitLocker, FileVault, LUKS) is necessary for complete hardware-level storage sanitization.

---

## 2. Cryptographic Implementation

* **Symmetric Encryption**: Authenticated `AES-256-GCM` using 256-bit keys, 96-bit random nonces, and 128-bit GHASH authentication tags.
* **Key Derivation Function (KDF)**: `Argon2id` with cryptographically random 128-bit salts for master passphrase derivation.
* **Credential Storage**: Randomly generated 256-bit vault keys are persisted in the native operating system credential store (Windows Credential Manager / DPAPI, macOS Keychain, Linux Secret Service) under the service namespace `vessel_vault`.
* **Cryptographic Sanitization**: In-memory key buffers and uncompressed archives are cleared using the `zeroize` primitive after use.

---

## 3. Filesystem & Profile Isolation

Each container is bound to its own profile directory via `WebviewWindowBuilder::data_directory`:

* **Windows**: `%LOCALAPPDATA%\Vessel\profiles\<CONTAINER_ID>`
* **macOS**: `~/Library/Application Support/Vessel/profiles/<CONTAINER_ID>`
* **Linux**: `~/.local/share/Vessel/profiles/<CONTAINER_ID>`

---

## 4. IPC Shielding & Capability Scoping

Tauri v2 capability scoping strictly isolates webviews:

* **Manager Window (`manager`)**: Holds necessary permissions (`core:default`, `opener:default`) to list, create, launch, and pin containers.
* **Container Windows (`container-<ID>`)**: External webviews are opened with **zero Tauri capabilities**. Untrusted scripts executing inside remote web applications cannot invoke native Tauri IPC commands, filesystem APIs, or shell commands.

---

## 5. Content Security Policy (CSP)

The Manager GUI enforces a strict CSP restricting resources to local assets (`'self'`), fonts, and cached icons. External web content is strictly isolated within standalone container webviews and is never loaded into the Manager DOM.

---

## 6. Data Locality

All container profiles, configurations (`profiles.json`), and vaults reside exclusively on your local machine. Vessel has no external tracking servers and transmits zero telemetry.
