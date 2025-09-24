# Replay & Persistence Workflow

This guide explains how to inspect the spans emitted by the LogLine Folding Engine and replay a simulation from the JSONL logs.

## Run a Simulation

```bash
cargo run -- --preset demo --temp 305 --dt 2 --seed 42 \
  --env aqueous --anneal 305:290:500 --diamond-threshold -8.0 \
  --log logs/custom-output.jsonl
```

Key flags:
- `--temp`, `--dt`, `--seed`: control temperature (K), timestep (ms), and RNG seed.
- `--anneal start:end:steps`: linear temperature schedule (Metropolis annealing).
- `--env`: energy environment preset (`aqueous`, `vacuum`, `membrane`).
- `--diamond-threshold`: Gibbs free energy threshold for cataloging diamonds.
- `--log`: override span log path (defaults to `logs/output.jsonl`).

Simulation output includes:
- Total steps, convergence tick, final Gibbs free energy, informational efficiency.
- Span log path and diamond catalog path (if any).

## Inspect the JSONL Span Log

Spans are stored as JSON lines with the following schema:

```json
{
  "span_uuid": "...",
  "contract_id": "...",
  "span_label": "residue-3",
  "timestamp": "2025-01-19T02:11:42.615Z",
  "delta_theta": -12.3,
  "delta_E": -0.562,
  "delta_S": 0.021,
  "G": -0.568,
  "ghost_flag": false
}
```

- `delta_theta`: rotation angle (°) applied in this span.
- `delta_E`, `delta_S`, `G`: energy metrics (kcal/mol) for the step.
- `ghost_flag`: `true` when the span represents a rejected positive-energy move.

Violation records appear as:

```json
{"type":"violation","detail":"bond length out of range ..."}
```

## Replay a Log

```bash
cargo run -- --replay logs/output.jsonl --ghosts
```

Replay output:
- Contract/environment/temperature summary.
- Counts for applied vs. ghost spans, Metropolis acceptance stats.
- Final energy metrics and total work.
- Span-by-span detail when `--ghosts` is provided, highlighting `GHOST` entries. 

## Diamonds

When `G < threshold`, entries are persisted to `logs/diamonds.json` (or custom path). Each record includes:
- Contract name and provenance string (`contract::span_label`).
- Gibbs free energy, potential/kinetic energies, environment, temperature.
- Array of `Diamond` entries with span UUIDs that met the threshold.

Use these files to seed visualization, analytics, or machine-learning pipelines.
