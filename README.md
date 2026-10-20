# Vessel

Vessel is a minimal, decoupled desktop web app container engine built with Tauri v2 and SvelteKit. It turns web applications (such as ChatGPT, Linear, Slack, GitHub, and Notion) into standalone desktop apps, each running in a dedicated native window with an isolated profile directory.

---

## Architectural Design

Vessel implements a **Decoupled Manager-Container Architecture**:

1. **Manager Mode (`vessel`)**:
   - Running `vessel` without arguments boots the native Manager GUI.
   - Built to Raycast/Linear desktop standards using lightweight HTML, Tailwind CSS, and SvelteKit.
   - Manage installed web apps, trigger in-app launches, create native desktop shortcuts, and wipe sandboxes.

2. **Container Mode (`vessel --run-container <ID>`)**:
   - Running `vessel --run-container <ID>` boots **only** a native `WebviewWindow` pointing directly to the target external URL with its isolated data directory.
   - **The Manager DOM is never loaded or rendered in this mode**, keeping memory overhead minimal and startup instantaneous.
   - Single-instance IPC forwards secondary launches directly to the running process without socket overhead or process duplication.

---

## Profile & Storage Isolation

Cookies, cache, localStorage, indexedDB, and service workers are strictly partitioned per container on the filesystem:

* **Windows**: `%LOCALAPPDATA%\Vessel\profiles\<CONTAINER_ID>`
* **macOS**: `~/Library/Application Support/Vessel/profiles/<CONTAINER_ID>`
* **Linux**: `~/.local/share/Vessel/profiles/<CONTAINER_ID>`

Container profiles and cached icons are configured through:
* Registry: `<VESSEL_ROOT>/profiles.json`
* Cached Icons: `<VESSEL_ROOT>/icons/<CONTAINER_ID>.png`

---

## Core Features

* **Desktop Shortcuts**: Generate native `.lnk` shortcuts (Windows) or `.command` scripts (macOS) pointing directly to `vessel --run-container <ID>` with the container's cached icon.
* **Auto Title & Favicon Extraction**: Automatically fetches high-resolution site favicons and parses service titles when installing web apps.
* **Zero Bloat**: No simulated multi-window DOM systems, no background telemetry daemons, and no heavy runtime dependencies.
* **Strict Security Sandboxing**: Tauri IPC capabilities are scoped strictly to the Manager window; external container webviews have zero Tauri IPC permissions.

---

## Tech Stack

* **Frontend**: SvelteKit, TypeScript, Tailwind CSS, Vite
* **Backend**: Rust, Tauri v2, `tauri-plugin-single-instance`, `tauri-plugin-opener`

---

## Development

Install dependencies:

```bash
npm install
```

Run in development mode:

```bash
npm run tauri dev
```

Run frontend type and diagnostic checks:

```bash
npm run check
```

Run unit tests:

```bash
npm test
cd src-tauri && cargo test
```

Build production executable:

```bash
npm run tauri build
```

---

## License

This project is licensed under the MIT License.
