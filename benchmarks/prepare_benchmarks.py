#!/usr/bin/env python3
"""Download benchmark PDBs and emit accompanying metadata/contract stubs.

Usage:
    python3 benchmarks/prepare_benchmarks.py
"""
from __future__ import annotations

import hashlib
import json
import pathlib
import textwrap
import urllib.request
from urllib.error import URLError

ROOT = pathlib.Path(__file__).resolve().parents[1]
DATA_DIR = ROOT / "data" / "raw" / "benchmarks"
CONTRACT_DIR = ROOT / "contracts"
METADATA_DIR = ROOT / "data" / "processed" / "benchmarks"

TARGETS = {
    "trpcage": {
        "pdb_id": "1L2Y",
        "chain": "A",
        "contract": "trpcage_benchmark.lll",
    },
    "gb1": {
        "pdb_id": "1PGA",
        "chain": "A",
        "contract": "gb1_benchmark.lll",
    },
    "ww": {
        "pdb_id": "1E0L",
        "chain": "A",
        "contract": "ww_benchmark.lll",
    },
}

CONTRACT_TEMPLATE = """# {name} benchmark contract ({pdb_id})
set_physics_level GB
physics_span on

define_domain core 0-{length_minus_one}
require_chaperone Hsp70 for core
add_modification none at {length_minus_one}

span_alias initial_collapse
rotate residue=5 angle=-15 duration=5
rotate residue=9 angle=12 duration=5
commit

span_alias refinement
rotate residue=12 angle=-8 duration=4
rotate residue=15 angle=6 duration=4
commit
""".strip()


def download_pdb(pdb_id: str) -> pathlib.Path:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    path = DATA_DIR / f"{pdb_id}.pdb"
    if path.exists():
        return path
    url = f"https://files.rcsb.org/download/{pdb_id}.pdb"
    try:
        with urllib.request.urlopen(url) as response, path.open("wb") as handle:
            handle.write(response.read())
    except URLError as exc:  # pragma: no cover - network failure handling
        print(f"  ! failed to download {pdb_id}: {exc}")
        raise
    return path


def extract_sequence(pdb_path: pathlib.Path, chain: str) -> str:
    residues = []
    with pdb_path.open() as handle:
        for line in handle:
            if not line.startswith("SEQRES"):
                continue
            if len(line) < 12:
                continue
            if line[11].strip() != chain:
                continue
            residues.extend(line[19:].split())
    return "".join(residues)


def sequence_hash(sequence: str) -> str:
    return hashlib.sha1(sequence.encode("ascii", "ignore")).hexdigest()


def write_metadata(alias: str, info: dict, sequence: str, pdb_path: pathlib.Path) -> None:
    METADATA_DIR.mkdir(parents=True, exist_ok=True)
    data = {
        "alias": alias,
        "pdb_id": info["pdb_id"],
        "chain": info["chain"],
        "sequence": sequence,
        "sequence_hash": sequence_hash(sequence),
        "pdb_path": str(pdb_path.relative_to(ROOT)),
    }
    meta_path = METADATA_DIR / f"{alias}.json"
    with meta_path.open("w") as handle:
        json.dump(data, handle, indent=2)
    print(f"  metadata -> {meta_path.relative_to(ROOT)}")


def write_contract(alias: str, info: dict, sequence: str) -> None:
    CONTRACT_DIR.mkdir(parents=True, exist_ok=True)
    contract_text = CONTRACT_TEMPLATE.format(
        name=alias,
        pdb_id=info["pdb_id"],
        length_minus_one=max(len(sequence) - 1, 0),
    )
    contract_path = CONTRACT_DIR / info["contract"]
    with contract_path.open("w") as handle:
        handle.write(contract_text + "\n")
    print(f"  contract -> {contract_path.relative_to(ROOT)}")


def main() -> None:
    for alias, info in TARGETS.items():
        print(f"Preparing {alias} ({info['pdb_id']})...")
        pdb_path = download_pdb(info["pdb_id"])
        sequence = extract_sequence(pdb_path, info["chain"])
        write_metadata(alias, info, sequence, pdb_path)
        write_contract(alias, info, sequence)


if __name__ == "__main__":
    main()
