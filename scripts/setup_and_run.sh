#!/usr/bin/env bash

# Unified setup + benchmark runner for the LogLine Folding Engine.
#
# This script will:
#   1. Create (or reuse) a Conda environment with OpenMM and numpy.
#   2. Export the required environment variables for the physics bridge.
#   3. Prepare benchmark assets.
#   4. Run the benchmark contracts with optional --features openmm.
#   5. Optionally run the main application.
#
# Requirements:
#   - Conda/Miniforge installed (eg. `brew install miniforge`).
#   - Access to `cargo` (Rust toolchain).
#   - macOS or Linux shell.
#
# Usage:
#   scripts/setup_and_run.sh [--skip-openmm] [--conda-env LOG_LINE] [--features "openmm"] [--run TYPE]
#
# Flags:
#   --skip-openmm  Skip installing OpenMM (uses deterministic bridge fallback).
#   --conda-env    Name of conda environment to create/use (default: logline-openmm).
#   --features     Cargo features to pass (default: openmm).
#   --run          What to run: "benchmarks" (default), "app" (main application), "run-full" (complete setup and run), or "verify" (verify OpenMM setup).
#   --verbose      Enable verbose output for the application.
#   --input        Specify an input file for the application.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
PYTHON_BRIDGE_PATH="$ROOT_DIR/physics/openmm_bridge.py"
DEFAULT_CONDA_ENV="logline-openmm"
CARGO_FEATURES="openmm"
INSTALL_OPENMM=1
RUN_TYPE="benchmarks"
VERIFY_OPENMM=0
VERBOSE=0
INPUT_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-openmm)
      INSTALL_OPENMM=0
      shift
      ;;
    --conda-env)
      DEFAULT_CONDA_ENV="$2"
      shift 2
      ;;
    --features)
      CARGO_FEATURES="$2"
      shift 2
      ;;
    --run)
      RUN_TYPE="$2"
      shift 2
      ;;
    --verify)
      VERIFY_OPENMM=1
      shift
      ;;
    --verbose)
      VERBOSE=1
      shift
      ;;
    --input)
      INPUT_FILE="$2"
      shift 2
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

if ! command -v conda >/dev/null 2>&1; then
  echo "Conda/Miniforge not found. Install it first (eg. 'brew install miniforge')." >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo binary not found in PATH." >&2
  exit 1
fi

# Activate conda base
source "$(conda info --base)/etc/profile.d/conda.sh"

if conda env list | grep -q "^$DEFAULT_CONDA_ENV "; then
  echo "Using existing conda environment: $DEFAULT_CONDA_ENV"
  conda activate "$DEFAULT_CONDA_ENV"
else
  echo "Creating conda environment: $DEFAULT_CONDA_ENV"
  conda create -y -n "$DEFAULT_CONDA_ENV" python=3.10
  conda activate "$DEFAULT_CONDA_ENV"
fi

if [[ $INSTALL_OPENMM -eq 1 ]]; then
  echo "Installing OpenMM + dependencies via conda-forge..."
  conda install -y -c conda-forge openmm openmmforcefields numpy
else
  echo "Skipping OpenMM installation (--skip-openmm)."
fi

# Verify OpenMM is properly installed and configured
verify_openmm() {
  echo "=== Verifying OpenMM Setup ==="
  
  # Check OpenMM installation
  if python -c "import openmm" 2>/dev/null; then
    echo "✅ OpenMM is installed"
    OPENMM_VERSION=$(python -c "import openmm; print(openmm.__version__)")
    echo "   Version: $OPENMM_VERSION"
  else
    echo "❌ OpenMM is not installed properly"
    if [[ $INSTALL_OPENMM -eq 1 ]]; then
      echo "Re-installing OpenMM..."
      conda install -y -c conda-forge openmm openmmforcefields numpy
    else
      echo "Run: conda install -y -c conda-forge openmm openmmforcefields numpy"
      return 1
    fi
  fi

  # Check environment variables
  echo "PYTHON_OPENMM_BIN=$PYTHON_OPENMM_BIN"
  echo "OPENMM_BRIDGE_SCRIPT=$OPENMM_BRIDGE_SCRIPT"
  
  # Test the bridge script directly
  echo "Testing OpenMM bridge script..."
  if python "$OPENMM_BRIDGE_SCRIPT" --test 2>/dev/null; then
    echo "✅ OpenMM bridge script works correctly"
  else
    echo "❌ OpenMM bridge script test failed"
    return 1
  fi
  
  echo "OpenMM verification passed!"
  return 0
}

export PYTHON_OPENMM_BIN="$(which python)"
export OPENMM_BRIDGE_SCRIPT="$PYTHON_BRIDGE_PATH"

cd "$ROOT_DIR"

if [[ -n "$CARGO_FEATURES" ]]; then
  FEATURES_ARGS=("--features" "$CARGO_FEATURES")
else
  FEATURES_ARGS=()
fi

# Verify OpenMM if requested or if running with OpenMM
if [[ "$RUN_TYPE" == "verify" || ($VERIFY_OPENMM -eq 1 && "$CARGO_FEATURES" == *"openmm"*) ]]; then
  verify_openmm
  if [[ "$RUN_TYPE" == "verify" ]]; then
    exit $?
  fi
fi

# Run the requested component
if [[ "$RUN_TYPE" == "benchmarks" ]]; then
  echo "Running benchmarks..."
  python3 benchmarks/prepare_benchmarks.py
  mkdir -p logs/benchmarks
  python3 benchmarks/run_benchmarks.py "${FEATURES_ARGS[@]}"
  
  # Check if physics spans were generated with OpenMM
  if [[ "$CARGO_FEATURES" == *"openmm"* ]]; then
    echo "Checking for OpenMM physics spans in benchmark results..."
    LATEST_LOG=$(ls -t logs/benchmarks/*.jsonl | head -1)
    if [[ -f "$LATEST_LOG" ]]; then
      PHYSICS_COUNT=$(grep -o '"physics_span_count":[^,]*' "$LATEST_LOG" | head -1 | cut -d: -f2)
      echo "Physics span count: $PHYSICS_COUNT"
      if [[ "$PHYSICS_COUNT" -eq 0 ]]; then
        echo "⚠️  Warning: No physics spans detected. OpenMM might not be working correctly."
        echo "Run with --verify option to diagnose the issue."
      else
        echo "✅ OpenMM physics engine is working properly!"
      fi
    fi
  fi
  
  echo "Benchmarks completed. Logs available under logs/benchmarks/"
elif [[ "$RUN_TYPE" == "app" ]]; then
  echo "Running main application..."
  
  # Set additional args if provided
  RUN_ARGS=()
  if [[ -n "$INPUT_FILE" ]]; then
    if [[ ! -f "$INPUT_FILE" ]]; then
      echo "Input file not found: $INPUT_FILE" >&2
      exit 1
    fi
    RUN_ARGS+=("--input" "$INPUT_FILE")
    echo "Using input file: $INPUT_FILE"
  fi
  
  if [[ $VERBOSE -eq 1 ]]; then
    RUN_ARGS+=("--verbose")
  fi
  
  # Run with appropriate features
  if [[ -n "${FEATURES_ARGS[*]}" ]]; then
    echo "Running with features: ${FEATURES_ARGS[*]}"
    cargo run "${FEATURES_ARGS[@]}" -- "${RUN_ARGS[@]}"
  else
    cargo run -- "${RUN_ARGS[@]}"
  fi
  
  echo "Application execution completed."
  
elif [[ "$RUN_TYPE" == "run-full" ]]; then
  echo "Performing complete setup and running application..."
  
  # Ensure OpenMM is properly set up
  if [[ "$CARGO_FEATURES" == *"openmm"* ]]; then
    verify_openmm
    if [[ $? -ne 0 ]]; then
      echo "OpenMM verification failed. Fix the issues before running."
      exit 1
    fi
    echo "OpenMM setup verified successfully."
  fi
  
  # Set additional args if provided
  RUN_ARGS=()
  if [[ -n "$INPUT_FILE" ]]; then
    if [[ ! -f "$INPUT_FILE" ]]; then
      echo "Input file not found: $INPUT_FILE" >&2
      exit 1
    fi
    RUN_ARGS+=("--input" "$INPUT_FILE")
    echo "Using input file: $INPUT_FILE"
  fi
  
  if [[ $VERBOSE -eq 1 ]]; then
    RUN_ARGS+=("--verbose")
  fi
  
  # Run with appropriate features
  if [[ -n "${FEATURES_ARGS[*]}" ]]; then
    echo "Running with features: ${FEATURES_ARGS[*]}"
    cargo run "${FEATURES_ARGS[@]}" -- "${RUN_ARGS[@]}"
  else
    cargo run -- "${RUN_ARGS[@]}"
  fi
  
  echo "Full run completed successfully."
  
elif [[ "$RUN_TYPE" == "verify" ]]; then
  verify_openmm
else
  echo "Unknown run type: $RUN_TYPE" >&2
  echo "Supported types: benchmarks, app, run-full, verify" >&2
  exit 1
fi
