#!/usr/bin/env bash

# OpenMM verification script for LogLine Folding Engine
# This script checks if OpenMM is properly installed and configured

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
CONDA_ENV="logline-openmm"
PYTHON_BRIDGE_PATH="$ROOT_DIR/physics/openmm_bridge.py"

echo "=== LogLine OpenMM Verification Tool ==="

# Check if we're in the correct conda environment
CURRENT_ENV=$(conda info --envs | grep "\*" | awk '{print $1}')
if [[ "$CURRENT_ENV" != "$CONDA_ENV" ]]; then
    echo "❌ Not in the correct conda environment"
    echo "Current: $CURRENT_ENV"
    echo "Expected: $CONDA_ENV"
    echo "Run: conda activate $CONDA_ENV"
    exit 1
else
    echo "✅ Correct conda environment: $CONDA_ENV"
fi

# Check OpenMM installation
if python -c "import openmm" 2>/dev/null; then
    echo "✅ OpenMM is installed"
    OPENMM_VERSION=$(python -c "import openmm; print(openmm.__version__)")
    echo "   Version: $OPENMM_VERSION"
else
    echo "❌ OpenMM is not installed properly"
    echo "Run: conda install -y -c conda-forge openmm openmmforcefields numpy"
    exit 1
fi

# Check environment variables
PYTHON_BIN=$(which python)
echo "Python binary: $PYTHON_BIN"

if [[ -z "${PYTHON_OPENMM_BIN:-}" ]]; then
    echo "❌ PYTHON_OPENMM_BIN is not set"
    echo "Run: export PYTHON_OPENMM_BIN=\"\$(which python)\""
else
    echo "✅ PYTHON_OPENMM_BIN=$PYTHON_OPENMM_BIN"
    
    # Check if it points to the correct python
    if [[ "$PYTHON_OPENMM_BIN" != "$PYTHON_BIN" ]]; then
        echo "⚠️  Warning: PYTHON_OPENMM_BIN points to a different python than the current environment"
    fi
fi

if [[ -z "${OPENMM_BRIDGE_SCRIPT:-}" ]]; then
    echo "❌ OPENMM_BRIDGE_SCRIPT is not set"
    echo "Run: export OPENMM_BRIDGE_SCRIPT=\"$PYTHON_BRIDGE_PATH\""
else
    echo "✅ OPENMM_BRIDGE_SCRIPT=$OPENMM_BRIDGE_SCRIPT"
    
    # Check if the bridge script exists
    if [[ ! -f "$OPENMM_BRIDGE_SCRIPT" ]]; then
        echo "⚠️  Warning: OPENMM_BRIDGE_SCRIPT points to a file that doesn't exist"
    fi
fi

# Test the bridge script directly
echo
echo "Testing OpenMM bridge script..."
if python "$PYTHON_BRIDGE_PATH" --test 2>/dev/null; then
    echo "✅ OpenMM bridge script works correctly"
else
    echo "❌ OpenMM bridge script test failed"
fi

echo
echo "=== Fix Commands ==="
echo "# Run these commands to fix the environment:"
echo "conda activate $CONDA_ENV"
echo "conda install -y -c conda-forge openmm openmmforcefields numpy"
echo "export PYTHON_OPENMM_BIN=\"\$(which python)\""
echo "export OPENMM_BRIDGE_SCRIPT=\"$ROOT_DIR/physics/openmm_bridge.py\""
echo
echo "# Then run the benchmarks:"
echo "cd $ROOT_DIR"
echo "python3 benchmarks/prepare_benchmarks.py"
echo "python3 benchmarks/run_benchmarks.py --features openmm"
