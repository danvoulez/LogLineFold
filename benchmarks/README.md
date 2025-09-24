# Folding Benchmarks

This directory holds canonical targets for validating the physics-backed folding engine. Each target includes:

- **PDB download** (from RCSB) written under `data/raw/benchmarks/`.
- **`.lll` contract** capturing domains, chaperone hints, and physics level directives.
- **Metadata JSON** with sequence hash and reference information.

## Targets

| Alias     | PDB  | Notes                                  |
|-----------|------|----------------------------------------|
| `trpcage` | 1L2Y | Fast folder, small helix bundle        |
| `gb1`     | 1PGA | β-sheet domain, classic folding target |
| `ww`      | 1E0L | WW domain, multi-intermediate folding  |

## Preparing the Benchmarks

```bash
python3 benchmarks/prepare_benchmarks.py
```

The script downloads PDB files, extracts the primary sequence, computes a SHA-1 hash, and emits metadata alongside a suggested `.lll` contract. Contracts are placed under `contracts/`.

## Running a Benchmark (Example)

```bash
# Ensure the bridge knows where the OpenMM helper lives
export OPENMM_BRIDGE_SCRIPT="$(pwd)/physics/openmm_bridge.py"
export PYTHON_OPENMM_BIN=python3

# Generate benchmark assets
python3 benchmarks/prepare_benchmarks.py

# Run all benchmarks
python3 benchmarks/run_benchmarks.py --features openmm

# Or run everything (env + prep + run) via the orchestrator script
scripts/setup_and_run.sh

# (Na UI, o botão "Trigger Benchmarks" apenas chama este fluxo mockado e lembra você de rodar os comandos manualmente.)

# Run the folding engine with the benchmark contract
cargo run --features openmm -- \
  --contract contracts/trpcage_benchmark.lll \
  --log logs/trpcage_gb.jsonl
```

Each `logs/*.jsonl` can be replayed in the UI (`interface/awesome-ui`) to inspect energy/entropy timelines and spans. The metadata section also records `physics_level` and which spans invoked the physics bridge.

## Next Steps

- Replace `physics/openmm_bridge.py` with a real OpenMM implementation.
- Record trajectories (`.dcd/.xtc`) for every run and reference them in the span log.
- Publish benchmark bundles (`.tar.gz`) containing `.lll`, JSONL, PDB, and PyMOL scripts for reproducibilidade.
