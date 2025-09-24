# Data Directory Layout

```
data/
├── raw/        # Source FASTA/PDB files (read-only)
├── processed/  # Normalized chains, span baselines, cached hashes
└── README.md   # This file
```

## Provenance Workflow
1. Place downloaded datasets under `raw/` and record source URL + checksum.
2. Run preprocessing scripts (to be added) that emit normalized assets into `processed/`.
3. Record provenance entries in `docs/provenance_log.md` (created automatically when needed).
4. Use hashed filenames (`<protein>_<checksum>.pdb`) to guarantee immutability.

### Bundled demo dataset

- `raw/demo_genome.fa` — short FASTA sequence consumed by the dashboard's genome viewer. Replace with a project-specific FASTA and point the UI at it via the `GENOME_PATH` environment variable.
