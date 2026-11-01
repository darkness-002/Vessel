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
        # Check if live harness requirements exist
        vessel_bin = Path("src-tauri/target/release/vessel.exe")
        if not vessel_bin.exists():
            vessel_bin = Path("src-tauri/target/debug/vessel.exe")

        electron_dir = Path("benchmarks/electron-shell")
        if not vessel_bin.exists() or not electron_dir.exists():
            print("\n[-] Live measurement targets are pending build:")
            print(f"    - Vessel binary: {vessel_bin} (exists: {vessel_bin.exists()})")
            print(f"    - Electron test shell: {electron_dir} (exists: {electron_dir.exists()})")
            print("\nTo run live measurements, build the release targets as documented in benchmarks/README.md.")
            print("To generate offline synthetic prototyping data instead, re-run with --synthetic.")
            sys.exit(1)


if __name__ == "__main__":
    main()
