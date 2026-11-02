# Vessel Performance Benchmarking Suite

This directory contains the benchmarking harness and plotting suite for **Vessel** (Tauri v2 + Rust), evaluated against **Electron** and **Google Chrome**.

---

### IMPORTANT NOTICE: Real Measurements vs. Synthetic Models

* **Live Mode (Default)**: Measures actual running processes on real host hardware using `psutil`. Live execution launches the compiled Vessel application and comparative runtimes to collect real OS working-set telemetry.
* **Synthetic Prototyping (`--synthetic`)**: Runs an offline mathematical simulation based on target baseline models. All synthetic data files contain `SYNTHETIC` in their filename, print a console warning, and have watermark stamps on generated charts. Previously generated synthetic demonstration figures are stored in [`benchmarks/synthetic_example/`](./synthetic_example/).

---

## 1. Methodology & Metrics

### Evaluated Targets:
1. **Vessel (Tauri v2 + Rust)**: Dedicated webview processes per container profile. Unfocused containers are hibernated by the supervisor after inactivity, destroying the webview process and releasing memory to the host OS.
2. **Electron**: Dedicated Chromium/Node.js runtime per application instance.
3. **Google Chrome**: Standalone application profile instances (`--app=<URL> --user-data-dir=<DIR>`).

### Lifecycle Phases:
* **Phase A: Cold Launch (0 – 5s)**: Time from process invocation to initial navigation/render (ms).
* **Phase B: Active Usage (5 – 65s)**: Resident set size (RSS in MB) and CPU utilization during active interaction.
* **Phase C: Background / Inactivity (65 – 185s)**: Tests whether the runtime frees inactive resources.
* **Phase D: Wake & Refocus (185 – 210s)**: Time and memory required to wake or refocus the window.

---

## 2. Setup & Requirements

### 1. Python Environment:
```bash
pip install -r benchmarks/requirements.txt
```

### 2. Comparative Electron Test Shell:
Install the isolated Electron harness in `benchmarks/electron-shell`:
```bash
cd benchmarks/electron-shell
npm install
cd ../..
```

### 3. Vessel Binary:
Build the production release binary:
```bash
cd src-tauri
cargo build --release
cd ..
```

---

## 3. Running Benchmarks

### Live Empirical Measurement (Default):
```bash
python benchmarks/run_benchmarks.py --runs 10
```
Measures live working set (RSS MB) via `psutil`, cold start latency, and process tree CPU utilization across repeated trials ($\ge 10$), reporting mean, standard deviation, and 95% confidence intervals ($1.96 \cdot \sigma / \sqrt{n}$).
Outputs to `benchmarks/results/measured_benchmark_data.csv`.

> **Note on Baseline Data**: `src/lib/data/electron_baseline.json` is initially marked with `"source": "assumed"`. Once you run the live benchmark harness on your local machine, update `electron_baseline.json` with your measured values, hardware model, OS, and set `"source": "measured"`. Do not commit fabricated or unmeasured numbers.

### Synthetic Model Generation (Prototyping Only):
```bash
python benchmarks/run_benchmarks.py --synthetic --runs 5
```
Outputs to `benchmarks/synthetic_example/SYNTHETIC_benchmark_data.csv` with explicit synthetic watermarks and warnings.

---

## 4. Generating Figures

To generate figures from synthetic data:
```bash
python benchmarks/analyze_and_plot.py --synthetic
```

To generate figures from live empirical data:
```bash
python benchmarks/analyze_and_plot.py --input benchmarks/results/benchmark_data.csv
```
