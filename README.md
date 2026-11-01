# Vessel

Vessel is a lightweight desktop web application container engine built with **Tauri v2 (Rust)** and **SvelteKit (Svelte 5 with runes)**. It runs web applications (such as GitHub, Spotify, Slack, Linear, and Notion) in isolated native windows with encrypted profile vaults and idle resource hibernation.

---

## Architecture

Vessel follows a **Decoupled Manager-Container Architecture**:

1. **Manager Mode (`vessel`)**:
   - Running `vessel` without arguments launches the Manager dashboard.
   - Built with Svelte 5 runes, TypeScript, and Tailwind CSS.
   - Provides a launcher-first interface with dual densities (`minimal` default and `detailed`), status tracking, search filtering, encrypted vault controls, and telemetry visualization.

2. **Container Mode (`vessel --run-container <ID>`)**:
   - Running `vessel --run-container <ID>` boots **only** the target webview window with its isolated data directory.
   - **The Manager DOM is never loaded or rendered in container mode**, keeping memory overhead minimal and startup instantaneous.
   - Tauri's single-instance plugin forwards secondary launches to the running instance without process duplication.

---

## Core Features

* **Strict Profile Isolation**: Cookies, cache, localStorage, IndexedDB, and service workers are strictly partitioned on disk:
  * Windows: `%LOCALAPPDATA%\Vessel\profiles\<CONTAINER_ID>`
  * macOS: `~/Library/Application Support/Vessel/profiles/<CONTAINER_ID>`
  * Linux: `~/.local/share/Vessel/profiles/<CONTAINER_ID>`
* **Process Supervisor & Idle Hibernation**: A background task monitors window focus. When an unfocused container exceeds its inactivity threshold (default 15 minutes, configurable globally and per container), its OS webview process is destroyed, freeing working set RAM back to the operating system. Refocusing or clicking "Wake" recreates the window at the persisted route.
* **Encrypted Profile Vaults**: Profile data can be encrypted at rest using authenticated **AES-256-GCM** (with GHASH authentication tag) and **Argon2id** key derivation. Vault keys can be stored securely in the native OS credential store (Windows Credential Manager / DPAPI, macOS Keychain) or protected with a master passphrase.
* **Dual-Density Dashboard**: A single component tree supporting both `minimal` (clean launcher with status indicators and primary action) and `detailed` (live RAM/CPU sparklines, idle countdown meters, secondary actions, and baseline comparison).
* **Desktop Shortcuts**: Creates native `.lnk` shortcuts on Windows (or `.command` scripts on macOS) pointing directly to `vessel --run-container <ID>` with the application's cached icon.

---

## Limitations

* **Hibernation State Loss**: Hibernation reclaims host memory by terminating the underlying webview process (`win.destroy()`). This releases 100% of the working set memory, but **in-page state is lost** (e.g. playing audio streams, active video/voice calls, unsubmitted form inputs, and transient WebSockets). To prevent interruption, enable the **"Never hibernate"** toggle in the container's preferences menu for communication or media applications.
* **Protection at Rest Only**: Encrypted vaults protect profile data when sealed on disk (defending against offline filesystem inspection and cold backups). When a container is unlocked and actively running, its profile data is temporarily unpacked to disk so the native webview can operate on cookies and storage.
* **Windows-First Implementation**: Windows 10/11 with the Microsoft Edge WebView2 runtime is the primary platform. While macOS and Linux code paths exist for basic window creation and keyrings, features like Win32 process thread extraction and shortcut generation are tailored for Windows.

---

## Tech Stack

* **Frontend**: SvelteKit 2, Svelte 5 (runes: `$state`, `$derived`, `$effect`), TypeScript, Tailwind CSS, Vite
* **Backend**: Rust (2021 edition), Tauri v2, `sysinfo`, `aes-gcm`, `argon2`, `keyring`, `zeroize`
* **Test Suites**: Vitest (frontend unit tests), Cargo test (Rust backend cryptographic and supervisor test harness)

---

## Getting Started

### Prerequisites

* Node.js 18+ and npm
* Rust 1.77+ with Cargo
* Windows 10/11 with WebView2 runtime installed (standard on modern Windows)

### Installation

```bash
npm install
```

### Development

Run the frontend and Tauri backend concurrently:

```bash
npm run tauri dev
```

### Running Checks & Tests

Frontend type check:
```bash
npm run check
```

Frontend unit tests:
```bash
npm test
```

Backend test suite:
```bash
cd src-tauri && cargo test
```

Backend linter & formatter check:
```bash
cd src-tauri
cargo clippy
cargo fmt --check
```

### Production Build

```bash
npm run build
npm run tauri build
```

---

## License

This project is licensed under the MIT License.
