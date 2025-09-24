#!/usr/bin/env bash

# Quick run script for LogLine Folding Engine
# This is a simplified wrapper around setup_and_run.sh focused on running the application

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
SCRIPT_PATH="$ROOT_DIR/scripts/setup_and_run.sh"

# Default values
USE_OPENMM=1
VERIFY=1
INPUT_FILE=""
VERBOSE=0

# Process arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-openmm)
      USE_OPENMM=0
      shift
      ;;
    --no-verify)
      VERIFY=0
      shift
      ;;
    --input|-i)
      INPUT_FILE="$2"
      shift 2
      ;;
    --verbose|-v)
      VERBOSE=1
      shift
      ;;
    --help|-h)
      echo "Usage: run.sh [options]"
      echo
      echo "Options:"
      echo "  --no-openmm     Run without OpenMM physics engine"
      echo "  --no-verify     Skip OpenMM verification"
      echo "  --input|-i FILE Specify input file"
      echo "  --verbose|-v    Enable verbose output"
      echo "  --help|-h       Show this help message"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      echo "Use --help for usage information."
      exit 1
      ;;
  esac
done

# Build command
CMD_ARGS=("--run" "run-full")

# Add features
if [[ $USE_OPENMM -eq 1 ]]; then
  CMD_ARGS+=("--features" "openmm")
else
  CMD_ARGS+=("--skip-openmm")
fi

# Add verification if needed
if [[ $VERIFY -eq 1 && $USE_OPENMM -eq 1 ]]; then
  CMD_ARGS+=("--verify")
fi

# Add input file if specified
if [[ -n "$INPUT_FILE" ]]; then
  CMD_ARGS+=("--input" "$INPUT_FILE")
fi

# Add verbose flag if specified
if [[ $VERBOSE -eq 1 ]]; then
  CMD_ARGS+=("--verbose")
fi

# Run the command
"$SCRIPT_PATH" "${CMD_ARGS[@]}"
