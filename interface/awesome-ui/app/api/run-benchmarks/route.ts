import { NextResponse } from 'next/server'

export async function POST() {
  return NextResponse.json({
    status: 'manual-trigger-required',
    message:
      'Automatic execution is not available in this sandbox. Run `scripts/setup_and_run.sh` (or the individual benchmark commands) in a local terminal to execute the OpenMM benchmarks.',
    instructions: [
      'conda activate logline-openmm',
      'export OPENMM_BRIDGE_SCRIPT="$(pwd)/physics/openmm_bridge.py"',
      'export PYTHON_OPENMM_BIN="$(which python)"',
      'scripts/setup_and_run.sh --features openmm',
    ],
  })
}
