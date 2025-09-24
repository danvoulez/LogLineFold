# LogLine Folding Engine — Road to Publication

> **Goal:** deliver a reproducible, physics-backed folding pipeline with publishable benchmarks and audit-ready spans.

---

## Phase 1 · Physics Bridge (Weeks 1-2)
- Replace `physics/openmm_bridge.py` stub with a real OpenMM integration:
  - Amber99SB-ILDN + GBSA (implicit solvent) as the initial force field.
  - Langevin integrator, SHAKE constraints, configurable timestep.
  - Emit `span_simulation_chunk` metadata: `RMSD`, `Rg`, total energy, temperature, time (ps).
  - Save trajectories per span (`.dcd/.xtc`) and track their paths in the span log.
- Feature flag workflow (`--features openmm`) kept intact.

## Phase 2 · Canonical Benchmarks (Weeks 3-4)
- Automate runs for:
  - Trp-cage (1L2Y), GB1 (1PGA), WW domain (1E0L).
- Generate bundle per target:
  - `.lll` contract (domains + chaperone directives).
  - `.jsonl` span log, raw `.pdb` (start/end), `.dcd/.xtc`, PyMOL script.
- Metrics & validation:
  - RMSD vs native, Rg, folding time (<100 ns target for Trp-cage), intermediate state detection, reproducibility across seeds.
- Scripts:
  - `benchmarks/prepare_benchmarks.py`: download metadata + contracts.
  - `benchmarks/run_benchmarks.py`: execute contracts (`cargo run`) with optional `--features openmm`.

## Phase 3 · Reproducible Packages (Weeks 5-6)
- Publish benchmark bundles (`.tar.gz`) including scripts for replay (`cargo run -- --replay ...`).
- Document “How to Reproduce” (README + `benchmarks/` docs).
- UI enhancements:
  - Compare runs side-by-side.
  - Visualise RMSD/Rg curves, physics span coverage, domain-level folding status.

## Phase 4 · Domain & Chaperone Modelling (Weeks 7-8)
- Enforce domain-aware folding:
  - Use `define_domain` to drive per-domain metrics (convergence, ΔG, timelines).
  - Support co-translational scenarios (sequential domains, partial folding).
- Implement chaperone-specific behaviours:
  - `require_chaperone Hsp70` modifies energy acceptance during exposure events.
  - `GroEL` confinement via potential well or boundary sphere.

## Phase 5 · Experimental Alignment (Weeks 9-12)
- Integrate public datasets:
  - DisProt / LLPSDB (IDPs and phase separation benchmarks).
  - FOLD-2025 kinetic datasets for rate comparison.
- Validate predictions against experimental folding rates/phase diagrams.
- Draft whitepaper / companion publication with spans + trajectories for peer review.

---

### Continuous Tracks
- **Provenance & Safety:** static contract linting, execution hashes, biosafety filters.
- **Performance:** profile physics bridge, consider multi-GPU/multi-node scaling.
- **Developer Experience:** natural-language → `.lll` generator, REST API, CLI recipes.

This roadmap lives; update milestones as physics integration and benchmarks land.
