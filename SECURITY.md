# Security Model & Architecture

## 1. Process & Data Isolation
Vessel implements strict isolation between application instances to prevent session leakage and cross-site scripting (XSS) contamination.

- **Storage Isolation:** Each deployed app is assigned a unique `sessions_root` subdirectory. This ensures that cookies, localStorage, and indexedDB are completely partitioned. Gmail in "Vessel A" cannot see the cookies of Gmail in "Vessel B".
- **IPC Shielding:** The application enforces a strict Content Security Policy (CSP) that restricts the IPC bridge. Injected scripts are prevented from calling arbitrary Tauri commands.
- **Resource Monitoring:** Rust handles system-level metrics (CPU/RAM) using the `sysinfo` crate, keeping sensitive system data out of the JavaScript context.

## 2. Notification Capture
The "Brain" panel captures notifications by injecting a bridge script into the webview.
- **Privacy:** Notifications are stored in a local SQLite database (`vessel.db`) on your machine. 
- **Data Locality:** No notification data is ever transmitted to a central server. All processing is local.

## 3. Custom Script Injection
Users can inject custom CSS and JS. 
- **Risk:** This is a power-user feature. 
- **Safety:** Injection is scoped only to the specific app instance and is subject to the site's own security boundaries.

## 4. Encryption
- **Current State:** Data at rest (SQLite and JSON settings) is currently **unencrypted**. 
- **Roadmap:** Implementation of native keychain/os-level encryption for sensitive tokens is planned for the v0.2.0 milestone.

## 5. Hibernation Engine
To save resources, Vessel "freezes" background webviews.
- **Logic:** After a period of inactivity (default 15s), background processes are hibernated.
- **Safety:** State is preserved in memory but CPU cycles are zeroed until the tab is focused again.
