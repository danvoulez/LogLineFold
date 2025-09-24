#!/usr/bin/env bash

# OpenMM Fix Script for LogLine Folding Engine
# This script performs a comprehensive fix for common OpenMM integration issues

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
CONDA_ENV="logline-openmm"
PYTHON_BRIDGE_PATH="$ROOT_DIR/physics/openmm_bridge.py"
TEST_REQUEST_PATH=$(mktemp)

echo "===== LogLine OpenMM Fix Tool ====="

# Ensure we're in the right environment
source "$(conda info --base)/etc/profile.d/conda.sh"
if ! conda activate "$CONDA_ENV" 2>/dev/null; then
  echo "Creating conda environment: $CONDA_ENV"
  conda create -y -n "$CONDA_ENV" python=3.10
  conda activate "$CONDA_ENV"
else
  echo "✅ Using conda environment: $CONDA_ENV"
fi

# Force install OpenMM via conda-forge (not pip)
echo "Installing OpenMM via conda-forge..."
conda install -y -c conda-forge openmm openmmforcefields numpy

# Verify OpenMM is working
echo
echo "Verifying OpenMM installation..."
if python -c "import openmm, sys; print('OpenMM', openmm.__version__, 'Platform:', openmm.Platform.getPlatform(0).getName())" 2>/dev/null; then
  echo "✅ OpenMM is installed and working"
else
  echo "❌ OpenMM installation failed"
  exit 1
fi

# Set environment variables
echo
echo "Setting environment variables..."
export PYTHON_OPENMM_BIN="$(which python)"
export OPENMM_BRIDGE_SCRIPT="$PYTHON_BRIDGE_PATH"
echo "PYTHON_OPENMM_BIN=$PYTHON_OPENMM_BIN"
echo "OPENMM_BRIDGE_SCRIPT=$OPENMM_BRIDGE_SCRIPT"

# Create a test request
cat > "$TEST_REQUEST_PATH" << EOF
{
  "residues": 20,
  "model": "gbNeck2",
  "temperature": 310.0,
  "solvent": "water",
  "residue_sequence": "NLYIQWLKDGGPSSGRPPPS",
  "constraints": [
    { "type": "distance", "atoms": [1, 5], "distance": 10.0, "k": 10.0 },
    { "type": "angle", "atoms": [5, 10, 15], "angle": 1.57, "k": 5.0 }
  ]
}
EOF

# Test the bridge script directly
echo
echo "Testing OpenMM bridge with sample input..."
if python "$PYTHON_BRIDGE_PATH" < "$TEST_REQUEST_PATH"; then
  echo "✅ OpenMM bridge script processed input successfully"
else
  echo "❌ Bridge script failed with actual input"
  echo "Debugging with verbose output..."
  OPENMM_BRIDGE_DEBUG=1 python "$PYTHON_BRIDGE_PATH" < "$TEST_REQUEST_PATH" || true
fi

rm -f "$TEST_REQUEST_PATH"

# Check for OpenMM platforms
echo
echo "Available OpenMM platforms:"
python -c "import openmm; print('\n'.join([f'{i}: {openmm.Platform.getPlatform(i).getName()}' for i in range(openmm.Platform.getNumPlatforms())]))"

# Run benchmarks with verified setup
echo
echo "===== Running Benchmarks with Verified Setup ====="
echo "Preparing benchmark assets..."
cd "$ROOT_DIR"
python benchmarks/prepare_benchmarks.py
mkdir -p logs/benchmarks

echo "Running benchmarks with OpenMM feature..."
python benchmarks/run_benchmarks.py --features openmm

echo
echo "Checking benchmark results for physics flag..."
LATEST_LOG=$(ls -t logs/benchmarks/*.jsonl | head -1)
if [[ -f "$LATEST_LOG" ]]; then
  PHYSICS_COUNT=$(grep -o '"physics_span_count":[^,]*' "$LATEST_LOG" | head -1 | cut -d: -f2)
  echo "Physics span count: $PHYSICS_COUNT"
  if [[ "$PHYSICS_COUNT" -eq 0 ]]; then
    echo "❌ No physics spans detected. OpenMM integration is still not working."
    echo
    echo "Possible issues:"
    echo "1. The bridge script is not handling requests correctly"
    echo "2. There might be platform compatibility issues with OpenMM"
    echo "3. The benchmark might not be set to use OpenMM"
    
    echo
    echo "Detailed bridge debugging:"
    SAMPLE_INPUT=$(mktemp)
    echo '{"residues": 10, "model": "gbNeck2", "temperature": 310.0, "solvent": "water", "residue_sequence": "NLYIQWLKDG", "constraints": []}' > "$SAMPLE_INPUT"