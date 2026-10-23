# Vessel Automated Benchmarking & Academic Evaluation Suite

This directory contains the automated performance benchmarking harness and publication plotting suite for **Vessel** (Tauri v2 + Rust), evaluated against **Electron** and **Google Chrome** (standalone SSB / PWA profiles).

---

## 1. Experimental Methodology & Setup

### Evaluated Targets:
1. **Vessel (Tauri v2 + Rust)**:
   - Evaluated in two operational regimes:
     * **Active State**: Dedicated native `WebviewWindow` pointing directly to target web app with strictly partitioned filesystem profile (`data_directory`). The Manager DOM is never loaded or rendered in container mode.
     * **Hibernated State**: Intelligent background process supervisor terminates the underlying OS webview process upon idle timeout ($\ge 15\text{s}$ in testing, $15\text{min}$ in production), freeing all working set memory back to the OS while retaining a lightweight routing stub in memory.
2. **Electron (v32+)**:
   - Standard desktop web container bundling a dedicated Chromium engine, Node.js runtime, and V8 heap per instance.
3. **Google Chrome (v128+)**:
   - Chrome standalone application window (`chrome.exe --app=<URL> --user-data-dir=<PROFILE_DIR>`).

---

## 2. Benchmark Phases & Workloads

### Standard 4-Phase Operational Lifecycle:
* **Phase A: Cold Launch (0 – 5s)**: Measures time-to-interactive and initial process creation overhead (ms).
* **Phase B: Active Steady-State Usage (5 – 65s)**: Measures active CPU utilization and resident set size (RSS in MB) during typical user interactions.
* **Phase C: Background / Idle State (65 – 185s)**: Simulates window blur / loss of user focus. Tests whether the runtime detects inactivity and frees working set RAM.
* **Phase D: Wake & Refocus Event (185 – 210s)**: Measures wake restoration latency (ms) and re-instantiation memory footprint.

### Multi-Instance Concurrency Scaling ($N \in \{1, 3, 5\}$):
* **Workload 1**: Spotify Web (`https://open.spotify.com`)
* **Workload 2**: Linear (`https://linear.app`)
* **Workload 3**: Slack Web (`https://app.slack.com`)
* **Workload 4**: GitHub (`https://github.com`)
* **Workload 5**: Notion (`https://notion.so`)

---

## 3. Installation & Requirements

Ensure Python 3.10+ is available:

```bash
pip install -r benchmarks/requirements.txt
```

---

## 4. Running the Benchmarks

### Quick Mode (Rapid Verification):
```bash
python benchmarks/run_benchmarks.py --quick
```

### Full Benchmark (5 Iterations across all workloads):
```bash
python benchmarks/run_benchmarks.py --runs 5
```

Outputs raw telemetry samples to `benchmarks/results/benchmark_data.csv`.

---

## 5. Generating Publication-Quality Figures

Execute the analysis and visualization suite:

```bash
python benchmarks/analyze_and_plot.py
```

Generated Figures:
1. **`benchmarks/results/fig1_memory_timeline.png`**: Resident Working Set Memory (RSS in MB) over time across Vessel, Electron, and Chrome through all 4 lifecycle phases.
2. **`benchmarks/results/fig2_startup_latency.png`**: Cold launch latency comparison with variance error bars.
3. **`benchmarks/results/fig3_multi_instance_scaling.png`**: Multi-instance concurrency scaling curves from 1 to 5 simultaneous apps.
