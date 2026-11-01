#!/usr/bin/env python3
"""
Vessel Publication Plotting & Statistical Analysis Suite
-------------------------------------------------------
Generates publication-ready figures for IEEE / ACM formatted papers
and academic thesis reports:
  - `fig1_memory_timeline.png`: RAM (MB) vs. Time (s) across 4 operational phases
  - `fig2_startup_latency.png`: Cold Launch Latency (ms) with variance bars
  - `fig3_multi_instance_scaling.png`: Scaling curves across 1 to 5 concurrent apps
"""

import sys
import argparse
from pathlib import Path
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import seaborn as sns

# Academic Publication Style Configuration
plt.rcParams.update({
    "font.family": "serif",
    "font.size": 11,
    "axes.labelsize": 12,
    "axes.titlesize": 13,
    "axes.titleweight": "bold",
    "xtick.labelsize": 10,
    "ytick.labelsize": 10,
    "legend.fontsize": 10,
    "figure.titlesize": 14,
    "figure.dpi": 300,
    "savefig.dpi": 300,
    "savefig.bbox": "tight",
    "grid.alpha": 0.35,
    "grid.linestyle": "--",
    "axes.grid": True,
})

# Distinct, accessible academic color scheme
COLORS = {
    "Vessel": "#059669",        # Emerald green
    "Vessel (Hibernated)": "#0d9488", # Teal
    "Electron": "#dc2626",      # Crimson red
    "Google Chrome": "#2563eb", # Royal blue
}


def add_synthetic_watermark(fig):
    """Stamp a prominent watermark across figures generated from synthetic simulation data."""
    fig.text(
        0.5,
        0.5,
        "SYNTHETIC DATA - NOT MEASURED",
        fontsize=20,
        color="#dc2626",
        alpha=0.35,
        ha="center",
        va="center",
        rotation=28,
        weight="bold",
        bbox=dict(boxstyle="square,pad=0.5", facecolor="#fee2e2", alpha=0.3, edgecolor="#dc2626"),
        zorder=100,
    )


def plot_fig1_memory_timeline(df: pd.DataFrame, out_path: Path, is_synthetic: bool = False):
    """Figure 1: Resident Set Size (RSS Memory) Timeline across operational phases."""
    print("Generating Figure 1: Memory Consumption Timeline...")
    fig, ax = plt.subplots(figsize=(10, 5.5))

    subset = df[df["concurrency"] == 3]
    grouped = subset.groupby(["target", "timestamp_sec", "phase"])["memory_rss_mb"].agg(["mean", "std"]).reset_index()

    for target in ["Electron", "Google Chrome", "Vessel"]:
        data = grouped[grouped["target"] == target]
        color = COLORS[target]
        ax.plot(
            data["timestamp_sec"],
            data["mean"],
            label=f"{target} (N=3 Apps)",
            color=color,
            linewidth=2.2,
        )
        ax.fill_between(
            data["timestamp_sec"],
            data["mean"] - data["std"],
            data["mean"] + data["std"],
            color=color,
            alpha=0.12,
        )

    phases = [
        (0, 5, "Phase A\nCold Launch", "#f3f4f6"),
        (5, 65, "Phase B\nActive Steady-State", "#ffffff"),
        (65, 185, "Phase C\nIdle / Background Hibernation", "#f8fafc"),
        (185, 210, "Phase D\nWake Event", "#ffffff"),
    ]

    for start, end, label, bg_color in phases:
        ax.axvspan(start, end, color=bg_color, alpha=0.5, zorder=0)
        ax.axvline(x=start, color="#94a3b8", linestyle=":", linewidth=1.0)
        mid = (start + end) / 2.0
        ax.text(
            mid,
            ax.get_ylim()[1] * 0.94 if ax.get_ylim()[1] > 100 else 1150,
            label,
            ha="center",
            va="top",
            fontsize=8.5,
            fontweight="semibold",
            color="#475569",
            bbox=dict(boxstyle="round,pad=0.25", facecolor="white", edgecolor="#cbd5e1", alpha=0.85),
        )

    ax.annotate(
        "Vessel Hibernation Engine\nWebview Process Destroyed (0 MB)",
        xy=(80, 5),
        xytext=(95, 320),
        arrowprops=dict(facecolor="#059669", shrink=0.08, width=1.5, headwidth=7),
        fontsize=9,
        fontweight="bold",
        color="#065f46",
        bbox=dict(boxstyle="round,pad=0.3", facecolor="#ecfdf5", edgecolor="#a7f3d0"),
    )

    title = "Figure 1: Resident Working Set Memory (RSS) Over Operational Lifecycle"
    if is_synthetic:
        title += " [SYNTHETIC DATA]"
    ax.set_title(title)
    ax.set_xlabel("Elapsed Time (seconds)")
    ax.set_ylabel("Total Process Tree RSS Memory (MB)")
    ax.set_xlim(0, 210)
    ax.set_ylim(0, 1300)
    ax.legend(loc="upper left", framealpha=0.95, edgecolor="#cbd5e1")

    if is_synthetic:
        add_synthetic_watermark(fig)

    plt.tight_layout()
    plt.savefig(out_path)
    plt.close()
    print(f"  [+] Saved: {out_path}")


def plot_fig2_startup_latency(df: pd.DataFrame, out_path: Path, is_synthetic: bool = False):
    """Figure 2: Cold Launch Latency (ms) with variance bars."""
    print("Generating Figure 2: Cold Startup Latency Comparison...")
    fig, ax = plt.subplots(figsize=(7.5, 4.8))

    targets = ["Vessel", "Google Chrome", "Electron"]
    means = []
    stds = []
    colors = [COLORS[t] for t in targets]

    for t in targets:
        sub = df[(df["target"] == t) & (df["concurrency"] == 1)]["startup_time_ms"]
        means.append(sub.mean())
        stds.append(sub.std())

    bars = ax.bar(
        targets,
        means,
        yerr=stds,
        capsize=6,
        color=colors,
        edgecolor="#1e293b",
        linewidth=1.2,
        alpha=0.9,
        width=0.55,
    )

    for bar, mean in zip(bars, means):
        height = bar.get_height()
        ax.text(
            bar.get_x() + bar.get_width() / 2.0,
            height + 40,
            f"{mean:.1f} ms",
            ha="center",
            va="bottom",
            fontsize=10,
            fontweight="bold",
            color="#0f172a",
        )

    title = "Figure 2: Cold Application Startup Latency (Time-to-Interactive)"
    if is_synthetic:
        title += " [SYNTHETIC DATA]"
    ax.set_title(title)
    ax.set_ylabel("Startup Latency (milliseconds)")
    ax.set_ylim(0, max(means) * 1.3 if means else 2000)

    if is_synthetic:
        add_synthetic_watermark(fig)

    plt.tight_layout()
    plt.savefig(out_path)
    plt.close()
    print(f"  [+] Saved: {out_path}")


def plot_fig3_multi_instance_scaling(df: pd.DataFrame, out_path: Path, is_synthetic: bool = False):
    """Figure 3: Multi-Instance Concurrency Scaling Curves."""
    print("Generating Figure 3: Concurrency Scaling Behavior...")
    fig, ax = plt.subplots(figsize=(8.5, 5.0))

    concurrencies = [1, 3, 5]
    active_df = df[df["phase"] == "Active Usage"]
    idle_df = df[df["phase"] == "Idle / Background"]

    for target in ["Electron", "Google Chrome", "Vessel"]:
        means = [
            active_df[(active_df["target"] == target) & (active_df["concurrency"] == c)]["memory_rss_mb"].mean()
            for c in concurrencies
        ]
        color = COLORS[target]
        ax.plot(
            concurrencies,
            means,
            marker="o",
            markersize=7,
            linewidth=2.2,
            label=f"{target} (Active)",
            color=color,
        )

    vessel_hib_means = [
        idle_df[(idle_df["target"] == "Vessel") & (idle_df["concurrency"] == c)]["memory_rss_mb"].mean()
        for c in concurrencies
    ]
    ax.plot(
        concurrencies,
        vessel_hib_means,
        marker="s",
        markersize=7,
        linewidth=2.2,
        linestyle="--",
        label="Vessel (Hibernated)",
        color=COLORS["Vessel (Hibernated)"],
    )

    title = "Figure 3: Multi-App Working Set Memory Scaling Curves"
    if is_synthetic:
        title += " [SYNTHETIC DATA]"
    ax.set_title(title)
    ax.set_xlabel("Number of Concurrent Web Apps (N)")
    ax.set_ylabel("Total Working Set RSS Memory (MB)")
    ax.set_xticks(concurrencies)
    ax.legend(loc="upper left", framealpha=0.95, edgecolor="#cbd5e1")

    if is_synthetic:
        add_synthetic_watermark(fig)

    plt.tight_layout()
    plt.savefig(out_path)
    plt.close()
    print(f"  [+] Saved: {out_path}")


def main():
    parser = argparse.ArgumentParser(description="Vessel Benchmark Plotting Suite")
    parser.add_argument(
        "--input",
        type=str,
        default=None,
        help="Input CSV file path",
    )
    parser.add_argument(
        "--output-dir",
        type=str,
        default=None,
        help="Output directory for plots",
    )
    parser.add_argument(
        "--synthetic",
        action="store_true",
        help="Plot synthetic sample figures with explicit watermark",
    )
    args = parser.parse_args()

    if args.input:
        csv_file = Path(args.input)
    elif args.synthetic:
        csv_file = Path("benchmarks/synthetic_example/SYNTHETIC_benchmark_data.csv")
    else:
        csv_file = Path("benchmarks/results/benchmark_data.csv")

    is_synthetic = args.synthetic or "SYNTHETIC" in str(csv_file)

    if not csv_file.exists():
        if is_synthetic:
            print(f"[-] Synthetic data file not found: {csv_file}")
            print("    Run 'python benchmarks/run_benchmarks.py --synthetic' first.")
            sys.exit(1)
        else:
            print(f"[-] Real benchmark data file not found: {csv_file}")
            print("    Live measurements have not been recorded yet. See benchmarks/README.md.")
            print("    To view synthetic demonstration plots, run with --synthetic.")
            sys.exit(1)

    df = pd.read_csv(csv_file)
    print(f"[+] Loaded {len(df)} samples from: {csv_file}")

    if args.output_dir:
        results_dir = Path(args.output_dir)
    elif is_synthetic:
        results_dir = Path("benchmarks/synthetic_example")
    else:
        results_dir = Path("benchmarks/results")
    results_dir.mkdir(parents=True, exist_ok=True)

    prefix = "SYNTHETIC_" if is_synthetic else ""
    fig1_path = results_dir / f"{prefix}fig1_memory_timeline.png"
    fig2_path = results_dir / f"{prefix}fig2_startup_latency.png"
    fig3_path = results_dir / f"{prefix}fig3_multi_instance_scaling.png"

    plot_fig1_memory_timeline(df, fig1_path, is_synthetic=is_synthetic)
    plot_fig2_startup_latency(df, fig2_path, is_synthetic=is_synthetic)
    plot_fig3_multi_instance_scaling(df, fig3_path, is_synthetic=is_synthetic)

    print("\n" + "=" * 60)
    print(" ALL FIGURES GENERATED SUCCESSFULLY")
    print(f"  1. {fig1_path}")
    print(f"  2. {fig2_path}")
    print(f"  3. {fig3_path}")
    print("=" * 60)


if __name__ == "__main__":
    main()
