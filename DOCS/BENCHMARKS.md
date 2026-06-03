# Performance Benchmarks (Baseline)

## 1. Hibernation Impact
Vessel's hibernation engine reduces CPU usage of background containers to 0% by pausing the webview rendering loop.

| State | Avg. CPU (per container) | Avg. RAM (per container) |
|-------|--------------------------|--------------------------|
| **Active** | 2% - 15% (Site dependent) | 120 MB - 450 MB |
| **Hibernated** | **0.0%** | 80 MB - 300 MB |

*Note: RAM reduction occurs as the OS reclaims memory from the inactive webview process, though the process remains resident for instant wake-up.*

## 2. Startup & Cold Boot
- **Frontend Initialized:** ~400ms
- **First Container Ready:** ~1.2s
- **Disk Usage (Binary):** ~12MB (Compressed)

## 3. Benchmarking Methodology
To verify these numbers on your machine:
1. Open the **Diagnostics Panel** (Ctrl + D) in Vessel.
2. Monitor the **Resource Usage** bar.
3. Open a heavy site (e.g., YouTube) and switch to the Gallery.
4. Observe the CPU drop to 0.0% after the configured hibernation timeout (default 15s).
