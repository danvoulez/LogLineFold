#!/usr/bin/env python3
"""Utility invoked by the Rust CLI to derive lightweight embeddings via PyTorch.

The script intentionally keeps the dependency surface small: if PyTorch is not
available, we emit nothing and exit successfully so the Rust caller can fall
back to heuristic coordinates.
"""
from __future__ import annotations

import math
import sys

try:
    import torch  # type: ignore
except Exception:  # pragma: no cover - optional dependency
    sys.exit(0)


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        return 1
    sequence = argv[1]
    if not sequence:
        return 0

    # Represent each residue using a simple sinusoidal embedding so the Rust
    # side has deterministic features to work with.
    values = []
    for index, residue in enumerate(sequence):
        base = float(ord(residue.upper()) % 23)
        values.append(math.sin(base + index / 3.0) * 0.5)
        values.append(math.cos(base + index / 5.0) * 0.5)

    tensor = torch.tensor(values, dtype=torch.float32)
    tensor = torch.tanh(tensor)

    print(" ".join(f"{value:.6f}" for value in tensor.tolist()))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
