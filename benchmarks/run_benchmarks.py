#!/usr/bin/env python3
"""Run folding benchmarks with the LogLine engine.

Usage:
    python3 benchmarks/run_benchmarks.py [--features openmm]

The script executes each benchmark contract under `contracts/` and writes logs
under `logs/benchmarks/`. It assumes the binaries have already been built.
"""
from __future__ import annotations

import argparse
import pathlib
import subprocess
import sys
import shutil

ROOT = pathlib.Path(__file__).resolve().parents[1]
CONTRACTS = {
    "trpcage": "trpcage_benchmark.lll",
    "gb1": "gb1_benchmark.lll",
    "ww": "ww_benchmark.lll",
}
LOG_DIR = ROOT / "logs" / "benchmarks"


def cargo_run(args: argparse.Namespace, alias: str, contract: pathlib.Path) -> None:
    log_path = LOG_DIR / f"{alias}.jsonl"
    cmd = [
        "cargo",
        "run",
        "--release",
    ]
    if args.features:
        cmd.extend(["--features", args.features])
    cmd.extend([
        "--",
        "--contract",
        str(contract),
        "--log",
        str(log_path),
    ])
    if args.temperature is not None:
        cmd.extend(["--temp", str(args.temperature)])
    if args.extra:
        cmd.extend(args.extra)

    print(f"→ Running {alias}: {' '.join(cmd)}")
    subprocess.run(cmd, check=True, cwd=str(ROOT))
    print(f"  log → {log_path.relative_to(ROOT)}")


def ensure_contract(alias: str, contract_name: str) -> pathlib.Path:
    path = ROOT / "contracts" / contract_name
    if not path.exists():
        raise FileNotFoundError(
            f"Contract {contract_name} not found. Run benchmarks/prepare_benchmarks.py first."
        )
    return path


def main() -> int:
    parser = argparse.ArgumentParser(description="Run LogLine folding benchmarks")
    parser.add_argument(
        "--features",
        default=None,
        help="Optional Cargo feature list (e.g. 'openmm')",
    )
    parser.add_argument(
        "--temperature",
        type=float,
        default=None,
        help="Override temperature in Kelvin for all runs",
    )
    parser.add_argument(
        "extra",
        nargs=argparse.REMAINDER,
        help="Additional CLI args passed to folding-app",
    )
    args = parser.parse_args()

    if not shutil.which("cargo"):
        print("cargo binary not found in PATH.", file=sys.stderr)
        return 1

    LOG_DIR.mkdir(parents=True, exist_ok=True)

    for alias, contract_file in CONTRACTS.items():
        try:
            contract_path = ensure_contract(alias, contract_file)
        except FileNotFoundError as exc:
            print(f"Skipping {alias}: {exc}")
            continue
        try:
            cargo_run(args, alias, contract_path)
        except subprocess.CalledProcessError as exc:
            print(f"Benchmark {alias} failed with exit code {exc.returncode}", file=sys.stderr)
            return exc.returncode

    return 0


if __name__ == "__main__":
    import shutil

    sys.exit(main())
