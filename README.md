# LogLine Folding Engine v1

> **A computable, audit-ready protein folding stack that speaks `.lll`, logs every span, and can swap from toy solvers to full OpenMM in a single contract directive.**

---

## 🚀 Why This Exists

Classical MD pipelines are powerful but opaque: scripts scattered across Jupyter, binary trajectories without provenance, and zero ability to replay “what exactly happened”. The LogLine Folding Engine rethinks folding as an _executable contract_. Every move is a span, every span is auditable, and entire trajectories can be replayed, benchmarked, or handed to another lab with a single `.jsonl` file.

- **Computable contracts.** Express folding logic and domain/chaperone/PTM constraints in `.lll`.
- **Deterministic spans.** Each rotation is recorded with ΔS/ΔE, Gibbs, physics backend, and ghost status.
- **Swappable physics.** Contracts select `toy`, `coarse`, `GB`, or `full` solvers. Flip `physics_span on` to send spans through OpenMM.
- **Dashboard-native.** The bundled Next.js UI lets you browse runs, highlight physics spans, and inspect entropy/energy trajectories live.
- **Benchmark-ready.** One script pulls canonical PDB targets (Trp-cage, GB1, WW), writes metadata, and emits contracts wired for physics-backed execution.

---

## 📦 Repository Layout

```
workspace/
├── core/          # Folding runtime, parser, physics bridge, enforcement
├── time/          # Trajectories, entropy accounting, span graph
├── molecule/      # Residue data, energy model, parameters
├── interface/     # CLI, log integration, diamonds, shells
├── sim/           # Simulation metrics, visualisers
├── app/           # Binary entrypoint (`folding-app`)
├── physics/       # OpenMM bridge (Python stub, replace with real kernel)
├── benchmarks/    # Scripts + docs for canonical folding targets
└── contracts/     # `.lll` contracts (demo + benchmarks)
```

---

## ⚡ Quick Start (Toy Solver)

```bash
# 1) Run the demo preset
cargo run -- \
  --preset demo \
  --temp 305 \
  --dt 2 \
  --seed 1337 \
  --env aqueous \
  --anneal 305:290:500 \
  --log logs/output.jsonl

# 2) Replay (optional)
cargo run -- --replay logs/output.jsonl --ghosts

# 3) Inspect spans (JSONL first line = metadata, rest = spans / violations)
head -n 5 logs/output.jsonl
```

Key outputs:
- Deterministic span log (`logs/output.jsonl`)
- Diamond catalogue (`logs/diamonds.json`) when ΔG drops below the configured threshold
- CLI summary with acceptance rate, final Gibbs, informational efficiency

---

## 🧠 Contracts 101 (`.lll`)

| Directive | Example | Purpose |
|-----------|---------|---------|
| `rotate` | `rotate residue=5 angle=-12 duration=5` | Execute a rotation span |
| `clash_check` / `commit` / `rollback` | `commit` | Manage state + enforcement |
| `span_alias` | `span_alias helix_push` | Label upcoming spans |
| `define_domain` | `define_domain helixA 5-20` | Logical folding unit |
| `require_chaperone` | `require_chaperone Hsp70 for helixA` | Annotate helper requirements |
| `add_modification` | `add_modification phosphorylation at S50` | PTMs |
| `set_physics_level` | `set_physics_level GB` | Select solver fidelity (`toy`, `coarse`, `gb`, `full`) |
| `physics_span` | `physics_span on` | Toggle OpenMM bridge for subsequent spans |

Example block:

```lll
set_physics_level GB
physics_span on

define_domain helixA 5-20
require_chaperone Hsp70 for helixA

span_alias collapse_seed
rotate residue=5 angle=-12 duration=5
rotate residue=9 angle=6 duration=5
commit
```

---

## 🧪 Physics-Backed Mode

1. **Build with the OpenMM feature** (bridge currently a Python stub, ready for replacement):
   ```bash
   cargo run --features openmm -- --contract contracts/trpcage_benchmark.lll --log logs/trpcage.jsonl
   ```
2. **Bridge configuration**:
  ```bash
  export OPENMM_BRIDGE_SCRIPT="$(pwd)/physics/openmm_bridge.py"
  export PYTHON_OPENMM_BIN=python3
  ```
  Replace `physics/openmm_bridge.py` with your OpenMM implementation (Amber99SB-ILDN + implicit solvent recommended to start).
   A minimal setup with the bundled script looks like:

   ```bash
   pip install openmm numpy
   export PYTHON_OPENMM_BIN=python3
   export OPENMM_BRIDGE_SCRIPT="$(pwd)/physics/openmm_bridge.py"
   cargo run --features openmm -- --contract contracts/trpcage_benchmark.lll --log logs/trpcage.jsonl
   ```
3. **Span metadata** records:
   - `physics_level`: solver requested (`toy`, `coarse`, `gb`, `full`)
   - `physics_spans`: IDs of spans actually executed via the bridge
   - `physics_metrics`: per-span diagnostics (RMSD, radius of gyration, potential/kinetic energy, simulation time, optional trajectory path)

If the bridge declines or errors, the engine automatically falls back to the toy solver—ensuring runs never fail just because a high-fidelity backend is unavailable.

---

## 📊 Web Dashboard

```
cargo run -- --preset demo --log logs/ui-demo.jsonl
cd interface/awesome-ui
pnpm install
pnpm dev
```

- **Run selector**: pick any JSONL log.
- **Physics awareness**: header cards show the active physics backend and span coverage; span timelines badge physics spans.
- **Insights panel**: stabilising vs ghost hotspots, energy breakdown, and enforcement history.

The UI reads logs from `logs/` by default. Override via `LOGS_DIR` and `GENOME_PATH` environment variables if needed.

---

## 🧰 Benchmarks

Prepare canonical targets:

```bash
python3 benchmarks/prepare_benchmarks.py
python3 benchmarks/run_benchmarks.py --features openmm
# or run everything (env + prep + run) in one shot (requires conda)
scripts/setup_and_run.sh
```

> Tip: a UI button **Trigger Benchmarks** chama um endpoint mockado que lembra você de executar os comandos acima manualmente neste ambiente restrito.

Outputs:
- `data/raw/benchmarks/*.pdb` (Trp-cage 1L2Y, GB1 1PGA, WW 1E0L)
- `data/processed/benchmarks/*.json` with sequence hash + metadata
- `contracts/*_benchmark.lll` configured with physics directives and chaperone hints

Run one:

```bash
cargo run --features openmm -- \
  --contract contracts/trpcage_benchmark.lll \
  --log logs/trpcage_physics.jsonl
```

The resulting JSONL + PDB files form a reproducible bundle for publication or cross-lab validation. Extend the workflow with real OpenMM kernels to generate production-grade datasets (RMSD, Rg, ΔG trajectories, `.dcd/.xtc` snapshots, PyMOL scripts).

---

## 🔐 Provenance & Safety

- Deterministic spans with UUIDs, ΔS/ΔE, physics flags
- Ghost mode (`physics_span off`) for exploratory steps that don’t affect the committed trajectory
- Violations logged as structured JSON (`type`, `detail`)
- Diamonds (Gibbs below threshold) recorded for downstream analytics
- Biosafety scaffolding (sequence filters, audit spans, execution hashes) ready for extension

---

## 🛣️ Roadmap Snapshot

1. **Physics Bridge (Weeks 1-2)** – wire the OpenMM kernel (Amber99SB-ILDN + GBSA), log RMSD/Rg/energy per span chunk, dump `.dcd/.xtc` trajectories.
2. **Canonical Benchmarks (Weeks 3-4)** – automate Trp-cage, GB1, WW domain runs with reproducible `.lll` + `.jsonl` bundles.
3. **Reproducible Packages (Weeks 5-6)** – publish `.tar.gz` archives (contract, spans, PDB, PyMOL) and enhance the UI with run comparison dashboards.
4. **Domain & Chaperone Modelling (Weeks 7-8)** – leverage `define_domain`/`require_chaperone` for co-translational folding and GroEL/Hsp70-aware enforcement.
5. **Experimental Alignment (Weeks 9-12)** – integrate DisProt, LLPSDB, FOLD-2025 datasets and draft a publishable whitepaper with audit-ready trajectories.

See `docs/roadmap.md` for the detailed plan.

---

## 🧪 Testing

```bash
cargo test
```

Add `--features openmm` to validate the bridge integration once you wire a real kernel.

---

## 🤝 Contributing

1. Fork / clone the repo.  
2. `rustup target install wasm32-unknown-unknown` if you plan to extend the UI.  
3. `cargo fmt && cargo clippy` before submitting PRs.  
4. Update docs (`README`, `docs/lll_contract_spec.md`, `benchmarks/README.md`) when behaviour changes.

Want to generate `.lll` from natural language or add new contract directives? Open an issue—we’re building a fully programmable, auditable folding stack together.

---

Made with 🧬 + 🔭 for reproducible biocomputation.
