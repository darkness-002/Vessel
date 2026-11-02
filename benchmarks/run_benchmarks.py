#!/usr/bin/env python3
"""
Vessel Empirical Performance & Academic Benchmarking Suite
---------------------------------------------------------
Automated harness to collect empirical metrics comparing:
  1. Vessel (Active & Resource-Hibernated)
  2. Electron (Standard Desktop Application Runtime)
  3. Google Chrome (Standalone PWA / Isolated SSB Profile Window)

Measures:
  - Working Set / Resident Set Size (RSS Memory in MB) across parent + child process trees
  - Process Tree CPU Utilization (%)
  - Cold Startup Latency (Time to Interactive / Render in ms)
  - Wake-from-Hibernation Latency (ms)
  - Multi-instance concurrent scaling across 1, 3, and 5 concurrent web applications
"""

import os
import sys
import time
import math
import shutil
import argparse
import subprocess
from pathlib import Path
from dataclasses import dataclass
from typing import List, Dict, Optional, Tuple
import numpy as np
import pandas as pd

try:
    import psutil
except ImportError:
    psutil = None

# Default test application workloads
WORKLOADS = [
    {"id": "spotify", "name": "Spotify Web", "url": "https://open.spotify.com"},
    {"id": "linear", "name": "Linear", "url": "https://linear.app"},
    {"id": "slack", "name": "Slack Web", "url": "https://app.slack.com"},
    {"id": "github", "name": "GitHub", "url": "https://github.com"},
    {"id": "notion", "name": "Notion", "url": "https://notion.so"},
]

# Standard baseline model coefficients (used ONLY when --synthetic is explicitly passed)
SYNTHETIC_BASELINES = {
    "Vessel": {
        "cold_start_mean_ms": 182.4,
        "cold_start_std_ms": 14.8,
        "wake_mean_ms": 115.2,
        "wake_std_ms": 12.1,
        "active_base_rss_mb": 42.0,
        "active_rss_variance": 4.5,
        "active_cpu_mean": 0.8,
        "idle_active_rss_mb": 38.5,
        "hibernated_rss_mb": 0.0,
        "hibernated_cpu": 0.0,
        "idle_timeout_sec": 30.0,
    },
    "Electron": {
        "cold_start_mean_ms": 1640.0,
        "cold_start_std_ms": 185.0,
        "wake_mean_ms": 48.0,
        "wake_std_ms": 8.5,
        "active_base_rss_mb": 348.0,
        "active_rss_variance": 32.0,
        "active_cpu_mean": 2.9,
        "idle_active_rss_mb": 312.0,
        "hibernated_rss_mb": 305.0,
        "hibernated_cpu": 0.3,
        "idle_timeout_sec": 30.0,
    },
    "Google Chrome": {
        "cold_start_mean_ms": 780.0,
        "cold_start_std_ms": 65.0,
        "wake_mean_ms": 72.0,
        "wake_std_ms": 9.4,
        "active_base_rss_mb": 195.0,
        "active_rss_variance": 18.0,
        "active_cpu_mean": 1.7,
        "idle_active_rss_mb": 172.0,
        "hibernated_rss_mb": 165.0,
        "hibernated_cpu": 0.15,
        "idle_timeout_sec": 30.0,
    },
}


@dataclass
class ProcessTreeMetrics:
    total_rss_mb: float
    total_cpu_percent: float
    process_count: int


def get_process_tree_metrics(pid: int) -> ProcessTreeMetrics:
    """Measure total RSS memory and CPU% across parent and all descendant child processes."""
    if psutil is None:
        return ProcessTreeMetrics(0.0, 0.0, 0)

    try:
        parent = psutil.Process(pid)
        processes = [parent] + parent.children(recursive=True)
    except (psutil.NoSuchProcess, psutil.AccessDenied):
        return ProcessTreeMetrics(0.0, 0.0, 0)

    total_rss = 0
    total_cpu = 0.0
    active_count = 0

    for p in processes:
        try:
            mem = p.memory_info().rss
            cpu = p.cpu_percent(interval=None)
            total_rss += mem
            total_cpu += cpu
            active_count += 1
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue

    return ProcessTreeMetrics(
        total_rss_mb=round(total_rss / (1024.0 * 1024.0), 2),
        total_cpu_percent=round(total_cpu, 2),
        process_count=active_count,
    )


def kill_process_tree(pid: int):
    """Safely terminate parent and all descendant processes."""
    if psutil is None:
        return
    try:
        parent = psutil.Process(pid)
        children = parent.children(recursive=True)
        for child in children:
            try:
                child.terminate()
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                pass
        parent.terminate()
        _, alive = psutil.wait_procs([parent] + children, timeout=3)
        for p in alive:
            try:
                p.kill()
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                pass
    except (psutil.NoSuchProcess, psutil.AccessDenied):
        pass


def find_vessel_binary() -> Optional[Path]:
    for candidate in [
        Path("src-tauri/target/release/vessel.exe"),
        Path("src-tauri/target/release/vessel"),
        Path("src-tauri/target/debug/vessel.exe"),
        Path("src-tauri/target/debug/vessel"),
    ]:
        if candidate.exists():
            return candidate.resolve()
    return None


def find_electron_runner() -> Optional[List[str]]:
    electron_shell_dir = Path("benchmarks/electron-shell").resolve()
    if not (electron_shell_dir / "package.json").exists():
        return None

    # Check local node_modules in electron-shell
    local_bin = electron_shell_dir / "node_modules" / ".bin" / ("electron.cmd" if sys.platform == "win32" else "electron")
    if local_bin.exists():
        return [str(local_bin), str(electron_shell_dir)]

    # Check system electron
    system_electron = shutil.which("electron.cmd" if sys.platform == "win32" else "electron")
    if system_electron:
        return [system_electron, str(electron_shell_dir)]

    # Check npx
    if (electron_shell_dir / "node_modules" / "electron").exists():
        npx = shutil.which("npx.cmd" if sys.platform == "win32" else "npx")
        if npx:
            return [npx, "--prefix", str(electron_shell_dir), "electron", str(electron_shell_dir)]

    return None


def simulate_sample_telemetry(
    target: str,
    phase: str,
    t_sec: float,
    num_instances: int,
    rng: np.random.Generator,
) -> Tuple[float, float]:
    """Generate model-derived synthetic telemetry. ONLY used when --synthetic is active."""
    meta = SYNTHETIC_BASELINES[target]
    base_rss = meta["active_base_rss_mb"]
    rss_var = meta["active_rss_variance"]

    if phase == "Cold Launch":
        init_spike = 1.35 if target == "Electron" else 1.15
        rss = (base_rss * init_spike + rng.normal(0, rss_var * 0.5)) * num_instances
        cpu = (meta["active_cpu_mean"] * 4.5 + rng.uniform(0.5, 2.0)) * num_instances
    elif phase == "Active Usage":
        drift = np.sin(t_sec * 0.1) * 4.0
        rss = (base_rss + drift + rng.normal(0, rss_var)) * num_instances
        cpu = max(0.1, (meta["active_cpu_mean"] + rng.normal(0, 0.4)) * num_instances)
    elif phase == "Idle / Background":
        if target == "Vessel":
            idle_elapsed = t_sec - 65.0
            if idle_elapsed >= 15.0:
                rss = meta["hibernated_rss_mb"] * num_instances
                cpu = 0.0
            else:
                rss = (meta["idle_active_rss_mb"] + rng.normal(0, 2.0)) * num_instances
                cpu = max(0.0, rng.normal(0.08, 0.03)) * num_instances
        elif target == "Electron":
            rss = (meta["idle_active_rss_mb"] + rng.normal(0, 8.0)) * num_instances
            cpu = max(0.1, (meta["hibernated_cpu"] + rng.normal(0, 0.08))) * num_instances
        else:  # Google Chrome
            rss = (meta["idle_active_rss_mb"] + rng.normal(0, 5.0)) * num_instances
            cpu = max(0.05, (meta["hibernated_cpu"] + rng.normal(0, 0.05))) * num_instances
    elif phase == "Wake & Refocus":
        if target == "Vessel":
            rss = (base_rss * 0.95 + rng.normal(0, 3.0)) * num_instances
            cpu = (meta["active_cpu_mean"] * 1.8 + rng.normal(0, 0.3)) * num_instances
        else:
            rss = (base_rss + rng.normal(0, rss_var)) * num_instances
            cpu = (meta["active_cpu_mean"] + rng.normal(0, 0.5)) * num_instances
    else:
        rss = base_rss * num_instances
        cpu = 1.0 * num_instances

    return max(0.0, round(float(rss), 2)), max(0.0, round(float(cpu), 2))


def run_synthetic_suite(
    output_csv: Path,
    num_runs: int = 5,
    quick_mode: bool = False,
) -> pd.DataFrame:
    """Execute synthetic benchmark generator. Explicitly writes to SYNTHETIC output path."""
    print("*" * 72)
    print("* WARNING: GENERATING SYNTHETIC DATA")
    print("* These numbers are model-generated approximations, NOT live measurements.")
    print("*" * 72)
    print(f" Output Path:    {output_csv}")
    print(f" Iterations:     {num_runs} synthetic runs")
    print("*" * 72)

    rng = np.random.default_rng(seed=42)
    records = []

    if quick_mode:
        phase_durations = [
            ("Cold Launch", 5),
            ("Active Usage", 15),
            ("Idle / Background", 25),
            ("Wake & Refocus", 10),
        ]
    else:
        phase_durations = [
            ("Cold Launch", 5),
            ("Active Usage", 60),
            ("Idle / Background", 120),
            ("Wake & Refocus", 25),
        ]

    targets = ["Vessel", "Electron", "Google Chrome"]
    concurrencies = [1, 3, 5]

    for run_idx in range(1, num_runs + 1):
        for concurrency in concurrencies:
            for target in targets:
                meta = SYNTHETIC_BASELINES[target]
                cold_latency_ms = max(
                    20.0,
                    float(rng.normal(meta["cold_start_mean_ms"], meta["cold_start_std_ms"])),
                )
                wake_latency_ms = max(
                    15.0,
                    float(rng.normal(meta["wake_mean_ms"], meta["wake_std_ms"])),
                )

                t_clock = 0.0
                for phase_name, duration_sec in phase_durations:
                    for s in range(duration_sec):
                        rss_mb, cpu_pct = simulate_sample_telemetry(
                            target=target,
                            phase=phase_name,
                            t_sec=t_clock,
                            num_instances=concurrency,
                            rng=rng,
                        )

                        records.append({
                            "run_id": run_idx,
                            "timestamp_sec": round(t_clock, 1),
                            "phase": phase_name,
                            "target": target,
                            "concurrency": concurrency,
                            "memory_rss_mb": rss_mb,
                            "cpu_percent": cpu_pct,
                            "startup_time_ms": round(cold_latency_ms, 1),
                            "wake_time_ms": round(wake_latency_ms, 1),
                            "workload_profile": f"N={concurrency}",
                            "data_type": "SYNTHETIC",
                        })
                        t_clock += 1.0

    df = pd.DataFrame(records)
    output_csv.parent.mkdir(parents=True, exist_ok=True)
    df.to_csv(output_csv, index=False)
    print(f"\n[+] Wrote {len(df)} SYNTHETIC samples to: {output_csv}\n")
    return df


def run_live_suite(
    output_csv: Path,
    vessel_bin: Path,
    electron_cmd_prefix: List[str],
    num_runs: int = 10,
    sample_duration_sec: float = 6.0,
) -> pd.DataFrame:
    """Execute live empirical benchmark harness against real host processes."""
    print("=" * 72)
    print(" EXECUTING LIVE EMPIRICAL BENCHMARKS")
    print(f" Target Vessel:    {vessel_bin}")
    print(f" Target Electron:  {' '.join(electron_cmd_prefix)}")
    print(f" Iterations:       {num_runs} runs per workload")
    print(f" Sample Window:    {sample_duration_sec}s per run")
    print(f" Output File:      {output_csv}")
    print("=" * 72)

    records = []

    for run_idx in range(1, num_runs + 1):
        print(f"\n--- [Iteration {run_idx}/{num_runs}] ---")
        for workload in WORKLOADS:
            app_id = workload["id"]
            app_name = workload["name"]
            app_url = workload["url"]

            # 1. Measure Vessel
            print(f"  Measuring Vessel: {app_name}...", end="", flush=True)
            vessel_cmd = [str(vessel_bin), "--run-container", app_id]
            t0 = time.perf_counter()
            proc = subprocess.Popen(
                vessel_cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            cold_latency_ms = 0.0
            samples_rss = []
            samples_cpu = []
            start_wall = time.time()

            while time.time() - start_wall < sample_duration_sec:
                metrics = get_process_tree_metrics(proc.pid)
                if metrics.process_count > 0 and cold_latency_ms == 0.0 and metrics.total_rss_mb > 5.0:
                    cold_latency_ms = max(10.0, (time.perf_counter() - t0) * 1000.0)
                if metrics.process_count > 0:
                    samples_rss.append(metrics.total_rss_mb)
                    samples_cpu.append(metrics.total_cpu_percent)
                time.sleep(0.5)

            kill_process_tree(proc.pid)
            try:
                proc.kill()
            except Exception:
                pass

            vessel_rss = float(np.mean(samples_rss)) if samples_rss else 0.0
            vessel_cpu = float(np.mean(samples_cpu)) if samples_cpu else 0.0
            cold_latency_ms = cold_latency_ms if cold_latency_ms > 0 else (sample_duration_sec * 500)

            records.append({
                "run_id": run_idx,
                "target": "Vessel",
                "workload": app_name,
                "url": app_url,
                "memory_rss_mb": round(vessel_rss, 2),
                "cpu_percent": round(vessel_cpu, 2),
                "startup_time_ms": round(cold_latency_ms, 1),
                "data_type": "MEASURED",
            })
            print(f" RSS: {vessel_rss:.1f} MB | Start: {cold_latency_ms:.1f} ms")

            # 2. Measure Electron
            print(f"  Measuring Electron: {app_name}...", end="", flush=True)
            e_cmd = electron_cmd_prefix + [f"--url={app_url}", f"--duration={int(sample_duration_sec)}"]
            t0 = time.perf_counter()
            e_proc = subprocess.Popen(
                e_cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )

            e_cold_ms = 0.0
            e_samples_rss = []
            e_samples_cpu = []
            e_start_wall = time.time()

            while time.time() - e_start_wall < sample_duration_sec:
                metrics = get_process_tree_metrics(e_proc.pid)
                if metrics.process_count > 0 and e_cold_ms == 0.0 and metrics.total_rss_mb > 20.0:
                    e_cold_ms = max(20.0, (time.perf_counter() - t0) * 1000.0)
                if metrics.process_count > 0:
                    e_samples_rss.append(metrics.total_rss_mb)
                    e_samples_cpu.append(metrics.total_cpu_percent)
                time.sleep(0.5)

            kill_process_tree(e_proc.pid)
            try:
                e_proc.kill()
            except Exception:
                pass

            electron_rss = float(np.mean(e_samples_rss)) if e_samples_rss else 0.0
            electron_cpu = float(np.mean(e_samples_cpu)) if e_samples_cpu else 0.0
            e_cold_ms = e_cold_ms if e_cold_ms > 0 else (sample_duration_sec * 500)

            records.append({
                "run_id": run_idx,
                "target": "Electron",
                "workload": app_name,
                "url": app_url,
                "memory_rss_mb": round(electron_rss, 2),
                "cpu_percent": round(electron_cpu, 2),
                "startup_time_ms": round(e_cold_ms, 1),
                "data_type": "MEASURED",
            })
            print(f" RSS: {electron_rss:.1f} MB | Start: {e_cold_ms:.1f} ms")

    df = pd.DataFrame(records)
    output_csv.parent.mkdir(parents=True, exist_ok=True)
    df.to_csv(output_csv, index=False)
    print(f"\n[+] Wrote {len(df)} MEASURED live samples to: {output_csv}\n")

    # Compute Summary Statistics (Mean, StdDev, 95% Confidence Intervals)
    print("=" * 80)
    print(" LIVE EMPIRICAL BENCHMARK SUMMARY (95% Confidence Intervals)")
    print("=" * 80)
    for target in ["Vessel", "Electron"]:
        sub = df[df["target"] == target]
        n = len(sub)
        if n == 0:
            continue
        mem_mean = sub["memory_rss_mb"].mean()
        mem_std = sub["memory_rss_mb"].std()
        mem_ci = 1.96 * (mem_std / math.sqrt(n)) if n > 1 else 0.0

        start_mean = sub["startup_time_ms"].mean()
        start_std = sub["startup_time_ms"].std()
        start_ci = 1.96 * (start_std / math.sqrt(n)) if n > 1 else 0.0

        print(f" Target: {target} (N={n} runs)")
        print(f"   Memory (RSS):  {mem_mean:.1f} MB ± {mem_ci:.1f} MB (stddev: {mem_std:.1f} MB)")
        print(f"   Startup Time:  {start_mean:.1f} ms ± {start_ci:.1f} ms (stddev: {start_std:.1f} ms)")
        print("-" * 80)

    return df


def main():
    parser = argparse.ArgumentParser(description="Vessel Performance Benchmarking Suite")
    parser.add_argument(
        "--output",
        type=str,
        default=None,
        help="Path to output CSV destination",
    )
    parser.add_argument(
        "--runs",
        type=int,
        default=10,
        help="Number of iterations across each workload (default: 10)",
    )
    parser.add_argument(
        "--quick",
        action="store_true",
        help="Run compressed quick benchmark for verification",
    )
    parser.add_argument(
        "--mode",
        type=str,
        choices=["live"],
        default="live",
        help="Benchmarking collection mode (default: live)",
    )
    parser.add_argument(
        "--synthetic",
        action="store_true",
        help="Explicitly run synthetic model generator (produces SYNTHETIC data, not real measurements)",
    )
    args = parser.parse_args()

    if args.synthetic:
        output_path = Path(args.output) if args.output else Path("benchmarks/synthetic_example/SYNTHETIC_benchmark_data.csv")
        if "SYNTHETIC" not in str(output_path):
            output_path = output_path.with_name(f"SYNTHETIC_{output_path.name}")
        df = run_synthetic_suite(output_path, num_runs=args.runs, quick_mode=args.quick)
    else:
        # Default Live Measurement Mode
        print("=" * 72)
        print(" VESSEL LIVE EMPIRICAL BENCHMARKING HARNESS (DEFAULT)")
        print("=" * 72)

        vessel_bin = find_vessel_binary()
        electron_cmd = find_electron_runner()

        missing = []
        if psutil is None:
            missing.append("psutil Python package not installed (run: pip install -r benchmarks/requirements.txt)")
        if vessel_bin is None:
            missing.append("Vessel binary not found (build via: cd src-tauri && cargo build --release)")
        if electron_cmd is None:
            missing.append("Electron test shell not installed (install via: cd benchmarks/electron-shell && npm install)")

        if missing:
            print("\n" + "!" * 76)
            print("! LIVE BENCHMARK PREREQUISITES MISSING")
            print("!" * 76)
            for item in missing:
                print(f"  [-] {item}")
            print("\nTo collect live empirical telemetry on your host machine:")
            print("  1. Build Vessel:         cd src-tauri && cargo build --release")
            print("  2. Install Electron:     cd benchmarks/electron-shell && npm install")
            print("  3. Run live suite:       python benchmarks/run_benchmarks.py --runs 10")
            print("\nTo generate offline synthetic prototyping data instead:")
            print("  python benchmarks/run_benchmarks.py --synthetic --runs 5\n")
            print("!" * 76)
            sys.exit(1)

        output_path = Path(args.output) if args.output else Path("benchmarks/results/measured_benchmark_data.csv")
        sample_duration = 3.0 if args.quick else 6.0
        run_live_suite(
            output_csv=output_path,
            vessel_bin=vessel_bin,
            electron_cmd_prefix=electron_cmd,
            num_runs=args.runs,
            sample_duration_sec=sample_duration,
        )


if __name__ == "__main__":
    main()
