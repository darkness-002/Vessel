# Security & Isolation Model

Vessel implements a defense-in-depth isolation model designed to keep web applications, cookies, and authentication tokens strictly separated from one another and from the host operating system.

---

## 1. Filesystem & Profile Isolation

Each deployed web app container is bound to its own dedicated profile data directory using Tauri's native `WebviewWindowBuilder::data_directory`:

* **Windows**: `%LOCALAPPDATA%\Vessel\profiles\<CONTAINER_ID>`
* **macOS**: `~/Library/Application Support/Vessel/profiles/<CONTAINER_ID>`
* **Linux**: `~/.local/share/Vessel/profiles/<CONTAINER_ID>`

### Guarantees:
* **Cookie & Session Partitioning**: Browser sessions, cookies, localStorage, IndexedDB, and ServiceWorkers are physically partitioned per container on disk. A Slack container cannot read, modify, or leak session tokens belonging to a GitHub container.
* **Complete Deletion**: Deleting a container via the Manager immediately closes any active webview instance and recursively purges its entire profile data directory from the filesystem.

---

## 2. IPC Shielding & Capability Scoping

Tauri v2's capability security layer enforces strict window-level permission scoping:

* **Manager Window (`manager`)**: Holds permissions (`core:default`, `opener:default`) required to list, create, launch, and pin containers.
* **Container Windows (`container-<ID>`)**: Boot external web URLs (`WebviewUrl::External`) with **zero Tauri capabilities**. Untrusted scripts running inside remote websites cannot invoke native Tauri IPC commands, filesystem operations, or shell commands.

---

## 3. CSP & Content Restrictions

The Manager interface enforces a strict Content Security Policy (CSP):
* Restricts resource loading to local app assets (`'self'`), fonts, and high-resolution cached icons.
* External web content is strictly isolated within external container webviews and never rendered inside the Manager DOM.

---

## 4. Data Locality

All container profiles, configurations (`profiles.json`), and cached favicons reside locally on your machine. Vessel does not collect telemetry, track usage, or transmit user information to remote servers.
