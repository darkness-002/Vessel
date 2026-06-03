# IPC Interface & Type Contracts

This document defines the communication bridge between the Rust backend and the TypeScript frontend.

## 1. Rust Commands (`tauri::command`)

### System & Metrics
| Command | Payload | Response | Description |
|---------|---------|----------|-------------|
| `get_resource_usage` | `None` | `Result<ResourceUsage, String>` | Returns CPU and RAM usage for the main process. |
| `set_safe_mode` | `{ enabled: boolean }` | `void` | Toggles safe mode (disables custom JS/CSS). |

### Notifications & Diagnostics
| Command | Payload | Response | Description |
|---------|---------|----------|-------------|
| `get_notifications` | `{ limit?: number }` | `Result<VesselNotification[], String>` | Fetches stored notifications from SQLite. |
| `clear_notifications` | `None` | `Result<(), String>` | Clears the notification database. |
| `forward_notification` | `{ appId: string, title: string, body: string }` | `void` | Saves and broadcasts a new notification. |
| `report_webview_error`| `{ appId: string, webviewId?: string, category: string, message: string, detail?: string }` | `void` | Logs an error to the Diagnostics panel. |

### Webview & Tab Management
| Command | Payload | Response | Description |
|---------|---------|----------|-------------|
| `open_app` | `OpenAppPayload` (see below) | `Result<(), String>` | Creates or updates a webview instance. |
| `close_webview` | `{ id: string }` | `Result<(), String>` | Closes a specific webview and cleans up state. |
| `hide_all_webviews`| `None` | `Result<(), String>` | Hides all webviews and triggers hibernation. |
| `request_new_tab` | `{ appId: string, url: string, title?: string }` | `void` | Emits an event to open a new tab. |
| `delete_app_session`| `{ appId: string }` | `Result<(), String>` | Deletes all local session data for an app. |

---

## 2. Type Definitions (Shared)

### `OpenAppPayload` (Typescript Interface)
```typescript
interface OpenAppPayload {
  id: string;               // Unique webview identifier
  url: string;              // Target URL
  appId?: string;           // Root app configuration ID
  profile?: string;         // Session profile name
  css?: string;             // Base optimization CSS
  customCss?: string;       // User-defined CSS
  js?: string;              // Base optimization JS
  customJs?: string;        // User-defined JS
  injectionAllowlist?: string[]; // Allowed domains for custom injection
  idleSleepSeconds?: number; // Hibernation timeout
}
```

### `ResourceUsage` (Rust Struct)
```rust
struct ResourceUsage {
    cpu_percent: f32,
    ram_mb: u64,
}
```

---

## 3. Events (Backend → Frontend)

| Event Name | Payload Type | Description |
|------------|--------------|-------------|
| `vessel-notification` | `VesselNotification` | Fired when a new notification is captured. |
| `vessel-diagnostic` | `DiagnosticEvent` | Fired for system logs and webview errors. |
| `vessel-open-tab` | `NewTabRequest` | Fired when a webview requests a new tab (e.g., target="_blank"). |

---

## 4. Security & Validation
- **JSON Serialization:** All payloads are serialized via `serde` (Rust) and `JSON.stringify` (TS).
- **CamelCase:** The Rust backend uses `#[serde(rename_all = "camelCase")]` to match TypeScript conventions.
- **Validation:** JavaScript payloads are validated in Rust via `security::validate_js_payload` before injection.
