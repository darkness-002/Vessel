# Changelog

All notable changes to the Vessel codebase are documented in this file.

## [Unreleased] - 2026-11-02

### Phase 1: Honest Claims & Scientific Integrity
- **Baseline Data Model**: Extracted all hard-coded comparison values into `src/lib/data/electron_baseline.json` with schema fields (`source: "assumed"`, `hardware`, `os`, `date`, `runs`, and per-state statistics).
- **UI Framing**: Updated `MemoryHero.svelte`, `PerformanceRibbon.svelte`, `TopBar.svelte`, and `+page.svelte` to prominently label comparative metrics as *"Estimated vs. Electron baseline (assumed)"*. Removed fabricated "measured" claims from all user-facing views.
- **Synthetic Benchmark Guardrails**: Moved synthetic data to `benchmarks/synthetic_example/`, required an explicit `--synthetic` CLI flag in `run_benchmarks.py`, and added watermark stamps ("SYNTHETIC DATA - NOT MEASURED") across all generated charts.
- **Documentation Overhaul**: Completely rewrote `README.md` with accurate architecture descriptions (Manager mode vs Container mode), supervisor hibernation mechanics, encrypted vaults, and realistic platform limitations.
- **Threat Model**: Extended `SECURITY.md` with an explicit threat model detailing what encrypted vaults protect against (offline disk theft and cold backup inspection) and what they do not protect against (runtime memory scraping, user-level malware, or solid-state disk wear-leveling remnants).
- **Terminology Corrections**: Removed outdated working notes (`PROJECT_SUMMARY.txt`), corrected AES-256-GCM authentication mechanism from Poly1305 to GHASH, and corrected "hardware-backed" claims to "OS credential store".

### Phase 2: Cryptographic Hardening & Master Passphrase Vaults
- **VSSLVAULT2 Vault Format**: Introduced version 2 vault container format featuring an explicit mode byte (Mode 0: random 256-bit OS Keyring key; Mode 1: Master Passphrase derived via Argon2id). Retained full backward compatibility with legacy `VSSLVAULT1` files.
- **Durable Keyring Key Management**: Replaced `get_or_create_vault_key` with `create_and_store_key` and `load_key`. Keyring write operations are verified by reading back the stored secret; silent error suppression (`let _ =`) was removed. Plaintext profiles are never destroyed unless keys are durably stored.
- **Atomic Sealing & Decryption Verification**: Before wiping plaintext profile data, the vault archive is written to a temporary file, flushed with `fsync`, atomically renamed, and decrypted in memory to verify that its hash matches the original unencrypted tarball. Plaintext profiles are preserved if verification fails.
- **Cryptographic Memory Zeroization**: Replaced ineffective copy-and-zeroize patterns with `Zeroizing<[u8; 32]>` to ensure sensitive cryptographic keys and archive buffers are wiped from RAM.
- **Extensible Keyring Abstraction & Test Suite**: Abstracted the OS credential store behind the `KeyringProvider` trait. Added 10 tests verifying round trips in both modes, wrong passphrases, incorrect keys, tampered ciphertexts, truncated headers, invalid magic headers, keyring write failures, leftover crash files, and nonce uniqueness.
- **Dynamic Vault Unlock UI**: Updated `VaultUnlockModal.svelte` and TypeScript definitions to query vault modes; password inputs are only requested when a container is encrypted with a master passphrase.

### Phase 3: Persistent Configuration & Supervisor Policies
- **Persistent Global & Container Settings**: Added `settings.json` in the Vessel root directory for global preferences (`globalIdleThresholdMinutes`, `isDemoMode`). Extended `profiles.json` to store container overrides (`neverHibernate`, `idleThresholdMinutes`, and `profileLabel`).
- **Tauri IPC Commands**: Added `get_settings`, `update_settings`, and `update_container_policy` commands. The frontend store serves as a responsive local cache while the backend remains the authoritative source of truth.
- **Dynamic Supervisor Idle Monitor**: Configured `start_idle_monitor` to re-read settings and container overrides on each tick, strictly respecting `never_hibernate` toggles and per-container threshold overrides. Added supervisor unit tests for all policy variations.
- **Audio Playback Evaluation**: Investigated WebView2 `ICoreWebView2_8::IsDocumentPlayingAudio` COM vtable hooks. Documented in `README.md` Limitations why unmanaged COM dispatch without official Tauri crate bindings was omitted in favor of the explicit "Never hibernate" policy toggle.

### Phase 4: Real Process Telemetry & Live Benchmark Harness
- **WebView2 Process Tree Attribution**: Implemented process inspection in `src-tauri/src/supervisor.rs` to match child WebView2 processes by inspect command-line arguments (`--user-data-dir`). Aggregates working set memory and CPU utilization across browser, GPU, renderer, and utility processes.
- **Estimated Metric Fallback**: Added `estimated: bool` to `ContainerProcessMetrics`. If process attribution cannot inspect command-line arguments for an active container, it falls back to window PID/child splitting and labels the RAM measurement with an `estimated` tag in the UI.
- **Lingering Process Detection**: Updated hibernation telemetry to verify whether background child processes are still active, reporting true residual memory instead of hardcoding 0 MB.
- **Electron Benchmark Harness**: Created `benchmarks/electron-shell/` (`package.json` and `main.js`) providing a standalone Electron harness for empirical testing.
- **Live Benchmark Runner**: Updated `benchmarks/run_benchmarks.py` to measure cold start latency, working set memory, and CPU utilization across $\ge 10$ live runs with 95% confidence intervals ($1.96 \cdot \sigma / \sqrt{n}$). Added clear error messages and setup instructions when benchmark prerequisites are missing.

### Phase 5: UI Polish & CI Pipeline
- **Card Telemetry Badges**: Integrated small `estimated` badges into `ContainerCard.svelte` for both running and idle states when fallback metrics are used.
- **Card Action Loading States**: Verified and linked `isWaking`, `isUnlocking`, and `isLaunching` state trackers to card primary action buttons, rendering progress spinners and disabling redundant inputs.
- **Toast Notifications with Undo**: Confirmed interactive Undo toasts for both Hibernate and Vault Seal operations, allowing users to restore hibernated or sealed containers.
- **Unified Single-Tree Component Architecture**: Maintained a unified component tree with minimal density as the default and detailed density providing extra telemetry without swapping DOM hierarchies.
- **CI Workflow Hardening**: Updated `.github/workflows/ci.yml` to run `cargo fmt --check` and `cargo clippy -- -D warnings` on every push and pull request.
