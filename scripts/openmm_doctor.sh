#!/usr/bin/env bash

# OpenMM Doctor: Advanced diagnostics for LogLine Folding Engine's physics bridge
# Helps troubleshoot why OpenMM isn't being used even when installed

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
PYTHON_BRIDGE_PATH="$ROOT_DIR/physics/openmm_bridge.py"
CONDA_ENV="logline-openmm"

echo "===== LogLine OpenMM Doctor ====="
echo "Performing deep diagnostics on OpenMM setup..."

# Check if conda is available
if ! command -v conda >/dev/null 2>&1; then
  echo "❌ Conda not found in PATH"
  echo "Install conda first (e.g., brew install miniforge)"
  exit 1
fi

# Ensure we're in the right environment
source "$(conda info --base)/etc/profile.d/conda.sh"
CURRENT_ENV=$(conda info --envs | grep "\*" | awk '{print $1}')
if [[ "$CURRENT_ENV" != "$CONDA_ENV" ]]; then
  echo "❌ Not in $CONDA_ENV environment (current: $CURRENT_ENV)"
  echo "Activating $CONDA_ENV environment..."
  if ! conda activate "$CONDA_ENV" 2>/dev/null; then
    echo "❌ Failed to activate environment. Does it exist?"
    echo "Creating environment..."
    conda create -y -n "$CONDA_ENV" python=3.10
    conda activate "$CONDA_ENV"
  fi
else
  echo "✅ Correct conda environment: $CONDA_ENV"
fi

# Check Python and pip
PYTHON_BIN=$(which python)
echo "Using Python: $PYTHON_BIN"
PYTHON_VERSION=$(python --version)
echo "Python version: $PYTHON_VERSION"

# Check OpenMM installation status
echo "Checking OpenMM..."
if python -c "import openmm" 2>/dev/null; then
  echo "✅ OpenMM is installed"
  OPENMM_VERSION=$(python -c "import openmm; print(openmm.__version__)")
  echo "   Version: $OPENMM_VERSION"
  
  # Check where OpenMM is installed
  OPENMM_PATH=$(python -c "import openmm, os; print(os.path.dirname(openmm.__file__))")
  echo "   Path: $OPENMM_PATH"
else
  echo "❌ OpenMM is not installed"
  echo "Installing OpenMM..."
  conda install -y -c conda-forge openmm openmmforcefields numpy
fi

# Check NumPy installation
echo "Checking NumPy..."
if python -c "import numpy" 2>/dev/null; then
  echo "✅ NumPy is installed"
  NUMPY_VERSION=$(python -c "import numpy; print(numpy.__version__)")
  echo "   Version: $NUMPY_VERSION"
else
  echo "❌ NumPy is not installed"
  echo "Installing NumPy..."
  conda install -y -c conda-forge numpy
fi

# Check environment variables
echo "Checking environment variables..."
if [[ -z "${PYTHON_OPENMM_BIN:-}" ]]; then
  echo "❌ PYTHON_OPENMM_BIN is not set"
  echo "Setting it now..."
  export PYTHON_OPENMM_BIN="$PYTHON_BIN"
else
  echo "✅ PYTHON_OPENMM_BIN=$PYTHON_OPENMM_BIN"
  if [[ "$PYTHON_OPENMM_BIN" != "$PYTHON_BIN" ]]; then
    echo "⚠️  Warning: Points to a different Python than current ($PYTHON_BIN)"
    echo "Fixing..."
    export PYTHON_OPENMM_BIN="$PYTHON_BIN"
  fi
fi

if [[ -z "${OPENMM_BRIDGE_SCRIPT:-}" ]]; then
  echo "❌ OPENMM_BRIDGE_SCRIPT is not set"
  echo "Setting it now..."
  export OPENMM_BRIDGE_SCRIPT="$PYTHON_BRIDGE_PATH"
else
  echo "✅ OPENMM_BRIDGE_SCRIPT=$OPENMM_BRIDGE_SCRIPT"
fi

# Check bridge script
echo "Checking bridge script..."
if [[ ! -f "$PYTHON_BRIDGE_PATH" ]]; then
  echo "❌ Bridge script not found at: $PYTHON_BRIDGE_PATH"
  exit 1
else
  echo "✅ Bridge script exists"
  
  # Check permissions
  if [[ ! -r "$PYTHON_BRIDGE_PATH" ]]; then
    echo "❌ Bridge script is not readable"
    echo "Fixing permissions..."
    chmod +r "$PYTHON_BRIDGE_PATH"
  else
    echo "✅ Bridge script is readable"
  fi
  
  # Check if script has test mode
  if grep -q "\-\-test" "$PYTHON_BRIDGE_PATH"; then
    echo "✅ Bridge script has test mode"
  else
    echo "⚠️  Bridge script doesn't have a --test option"
  fi
  
  # Try to run the bridge script
  echo "Attempting to run bridge script..."
  if python "$PYTHON_BRIDGE_PATH" --test 2>/dev/null; then
    echo "✅ Bridge script executed successfully"
  else
    echo "❌ Bridge script failed"
    echo "Trying to debug the error..."
    python "$PYTHON_BRIDGE_PATH" --test || true
  fi
fi

# Test full integration
echo
echo "===== Testing Full Integration ====="
echo "Setting up a minimal test to verify bridge integration..."

# Create a simple test script
TEST_SCRIPT=$(mktemp)
cat > "$TEST_SCRIPT" <<EOL
import os
import sys
import subprocess

# Print environment info
print(f"Python: {sys.executable}")
print(f"PYTHON_OPENMM_BIN: {os.environ.get('PYTHON_OPENMM_BIN', 'Not set')}")
print(f"OPENMM_BRIDGE_SCRIPT: {os.environ.get('OPENMM_BRIDGE_SCRIPT', 'Not set')}")

# Try to import OpenMM
try:
    import openmm
    print(f"OpenMM version: {openmm.__version__}")
except ImportError:
    print("Failed to import OpenMM")
    sys.exit(1)

# Try to execute bridge script
bridge_script = os.environ.get('OPENMM_BRIDGE_SCRIPT')
if not bridge_script:
    print("OPENMM_BRIDGE_SCRIPT not set")
    sys.exit(1)

try:
    result = subprocess.run([sys.executable, bridge_script, "--test"], 
                          capture_output=True, text=True, check=True)
    print("Bridge test output:")
    print(result.stdout)
except subprocess.CalledProcessError as e:
    print("Bridge script failed:")
    print(e.stderr)
    sys.exit(1)

print("All tests passed!")
EOL

echo "Running integration test..."
python "$TEST_SCRIPT"
rm "$TEST_SCRIPT"

echo
echo "===== Fix Commands ====="
echo "# Add these to your shell startup file (e.g., ~/.bashrc or ~/.zshrc):"
echo "export PYTHON_OPENMM_BIN=\"$PYTHON_BIN\""
echo "export OPENMM_BRIDGE_SCRIPT=\"$PYTHON_BRIDGE_PATH\""
echo
echo "# Or run these before running benchmarks:"
echo "conda activate $CONDA_ENV"
echo "export PYTHON_OPENMM_BIN=\"\$(which python)\""
echo "export OPENMM_BRIDGE_SCRIPT=\"$ROOT_DIR/physics/openmm_bridge.py\""
echo
echo "# Then run benchmarks:"
echo "cd $ROOT_DIR"
echo "python benchmarks/prepare_benchmarks.py"
echo "python benchmarks/run_benchmarks.py --features openmm"
