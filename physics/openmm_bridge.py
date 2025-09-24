#!/usr/bin/env python3
"""Bridge between LogLine spans and OpenMM.

The script reads JSON requests from stdin and prints a JSON response on stdout.
If OpenMM is available it will run a coarse-grained simulation; otherwise it
falls back to a deterministic heuristic so the engine can continue to function
without the dependency.
"""
from __future__ import annotations

import json
import math
import sys
from dataclasses import dataclass
from typing import Any, Dict, List

try:  # pragma: no cover - optional dependency
    import numpy as np
except Exception:  # pragma: no cover - numpy not installed
    np = None
    HAS_NUMPY = False
else:
    HAS_NUMPY = True

try:  # pragma: no cover - optional dependency
    import openmm
    from openmm import unit
    from openmm import app

    HAS_OPENMM = True and HAS_NUMPY
except Exception:  # pragma: no cover - OpenMM not installed
    HAS_OPENMM = False


@dataclass
class Command:
    residue: int
    angle_degrees: float
    duration_ms: int
    label: str


@dataclass
class Request:
    level: str
    temperature: float
    residues: List[Dict[str, Any]]
    command: Command


@dataclass
class Response:
    applied_angle: float
    delta_entropy: float
    delta_information: float
    delta_energy: float
    gibbs_energy: float
    duration_ms: int
    rmsd: float
    radius_of_gyration: float
    potential_energy: float
    kinetic_energy: float
    temperature: float
    simulation_time_ps: float
    trajectory_path: str | None = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "applied_angle": self.applied_angle,
            "delta_entropy": self.delta_entropy,
            "delta_information": self.delta_information,
            "delta_energy": self.delta_energy,
            "gibbs_energy": self.gibbs_energy,
            "duration_ms": self.duration_ms,
            "rmsd": self.rmsd,
            "radius_of_gyration": self.radius_of_gyration,
            "potential_energy": self.potential_energy,
            "kinetic_energy": self.kinetic_energy,
            "temperature": self.temperature,
            "simulation_time_ps": self.simulation_time_ps,
            "trajectory_path": self.trajectory_path,
        }


LEVEL_FACTORS = {
    "toy": 0.5,
    "coarse": 0.75,
    "gb": 1.0,
    "full": 1.25,
}


def load_request() -> Request:
    payload = json.load(sys.stdin)
    command = Command(
        residue=int(payload["command"]["residue"]),
        angle_degrees=float(payload["command"]["angle_degrees"]),
        duration_ms=int(payload["command"].get("duration_ms", 1)),
        label=payload["command"].get("label", "span"),
    )
    return Request(
        level=payload.get("level", "toy"),
        temperature=float(payload.get("temperature", 300.0)),
        residues=payload.get("residues", []),
        command=command,
    )


def main() -> int:
    try:
        request = load_request()
        if HAS_OPENMM:
            response = compute_with_openmm(request)
        else:
            response = heuristic_response(request)
        json.dump(response.to_dict(), sys.stdout)
        return 0
    except Exception as exc:  # pragma: no cover - defensive logging
        json.dump({"error": str(exc)}, sys.stdout)
        return 1


def heuristic_response(request: Request) -> Response:
    factor = LEVEL_FACTORS.get(request.level.lower(), 1.0)
    applied_angle = request.command.angle_degrees * factor
    magnitude = abs(applied_angle)
    delta_entropy = 0.015 * magnitude * factor
    delta_information = 0.0075 * magnitude * factor
    delta_energy = 0.001 * magnitude * (request.temperature / 300.0) * factor
    gibbs_energy = delta_energy - request.temperature * delta_entropy * 0.001
    duration_ms = max(1, request.command.duration_ms)

    # Heuristic values for diagnostics keep contracts flowing even sem OpenMM
    rmsd = magnitude * 0.01
    radius_of_gyration = 1.5 + magnitude * 0.002
    potential_energy = delta_energy * 1000.0
    kinetic_energy = delta_energy * 800.0
    temperature = request.temperature
    simulation_time_ps = duration_ms * 0.01

    return Response(
        applied_angle=applied_angle,
        delta_entropy=delta_entropy,
        delta_information=delta_information,
        delta_energy=delta_energy,
        gibbs_energy=gibbs_energy,
        duration_ms=duration_ms,
        rmsd=rmsd,
        radius_of_gyration=radius_of_gyration,
        potential_energy=potential_energy,
        kinetic_energy=kinetic_energy,
        temperature=temperature,
        simulation_time_ps=simulation_time_ps,
        trajectory_path=None,
    )


def compute_with_openmm(request: Request) -> Response:
    if not HAS_NUMPY:
        raise RuntimeError("numpy is required for the OpenMM bridge")
    topology, system, initial_pos = build_coarse_system(request.residues, request.level)
    integrator = openmm.LangevinIntegrator(
        request.temperature * unit.kelvin,
        1.0 / unit.picosecond,
        2.0 * unit.femtoseconds,
    )
    simulation = app.Simulation(topology, system, integrator, openmm.Platform.getPlatformByName("Reference"))
    simulation.context.setPositions(initial_pos)

    steps = max(20, int(request.command.duration_ms * 10))
    simulation.step(steps)

    state = simulation.context.getState(getEnergy=True, getPositions=True, getVelocities=True)
    positions = state.getPositions(asNumpy=True)
    velocities = state.getVelocities(asNumpy=True)

    rmsd = compute_rmsd(initial_pos, positions)
    radius_gyration = compute_radius_of_gyration(positions)
    potential_energy = state.getPotentialEnergy().value_in_unit(unit.kilojoule_per_mole)
    kinetic_energy = state.getKineticEnergy().value_in_unit(unit.kilojoule_per_mole)
    dof = max(1, 3 * len(positions) - 6)
    temperature = (2 * kinetic_energy * 1000.0) / (
        dof * unit.MOLAR_GAS_CONSTANT_R.value_in_unit(unit.joule / (unit.kelvin * unit.mole))
    )
    dt_ps = simulation.integrator.getStepSize().value_in_unit(unit.picosecond)
    time_ps = steps * dt_ps

    delta_entropy = rmsd * 0.02
    delta_information = radius_gyration * 0.01
    delta_energy = potential_energy * 0.001
    gibbs_energy = delta_energy - (temperature * delta_entropy * 0.001)
    applied_angle = request.command.angle_degrees

    return Response(
        applied_angle=applied_angle,
        delta_entropy=delta_entropy,
        delta_information=delta_information,
        delta_energy=delta_energy,
        gibbs_energy=gibbs_energy,
        duration_ms=max(1, request.command.duration_ms),
        rmsd=rmsd,
        radius_of_gyration=radius_gyration,
        potential_energy=potential_energy,
        kinetic_energy=kinetic_energy,
        temperature=temperature,
        simulation_time_ps=time_ps,
        trajectory_path=None,
    )


def build_coarse_system(residues: List[Dict[str, Any]], level: str):
    n = max(1, len(residues))
    topology = app.Topology()
    chain = topology.addChain()
    system = openmm.System()
    bond_force = openmm.HarmonicBondForce()
    system.addForce(bond_force)

    sigma = 0.35 * unit.nanometer
    epsilon = 0.05 * unit.kilojoule_per_mole
    nbforce = openmm.CustomNonbondedForce(
        "4*sqrt(epsilon1*epsilon2)*((0.5*(sigma1+sigma2)/r)^12 - (0.5*(sigma1+sigma2)/r)^6)"
    )
    nbforce.addPerParticleParameter("sigma")
    nbforce.addPerParticleParameter("epsilon")
    nbforce.setNonbondedMethod(openmm.CustomNonbondedForce.CutoffNonPeriodic)
    nbforce.setCutoffDistance(1.2 * unit.nanometer)
    system.addForce(nbforce)

    positions = []
    previous_atom = None
    for i in range(n):
        residue = topology.addResidue("GLY", chain)
        element = app.Element.getBySymbol("C")
        atom = topology.addAtom(f"C{i}", element, residue)
        system.addParticle(110.0 * unit.amu)
        nbforce.addParticle([sigma.value_in_unit(unit.nanometer), epsilon.value_in_unit(unit.kilojoule_per_mole)])
        if previous_atom is not None:
            topology.addBond(previous_atom, atom)
            bond_force.addBond(
                i - 1,
                i,
                0.38 * unit.nanometer,
                stiffness_for_level(level),
            )
        previous_atom = atom

        pos = residues[i].get("position") if i < len(residues) else None
        if not pos or all(abs(float(c)) < 1e-6 for c in pos):
            vec = openmm.Vec3(float(i) * 0.38, 0.0, 0.0) * unit.nanometer
        else:
            vec = openmm.Vec3(float(pos[0]), float(pos[1]), float(pos[2])) * unit.angstrom
        positions.append(vec)

    return topology, system, positions


def stiffness_for_level(level: str) -> float:
    level = level.lower()
    if level == "toy":
        return 30.0 * unit.kilojoule_per_mole / (unit.nanometer**2)
    if level == "coarse":
        return 60.0 * unit.kilojoule_per_mole / (unit.nanometer**2)
    if level == "gb":
        return 90.0 * unit.kilojoule_per_mole / (unit.nanometer**2)
    return 120.0 * unit.kilojoule_per_mole / (unit.nanometer**2)


def compute_rmsd(initial, final):
    init = np.array([vec.value_in_unit(unit.nanometer) for vec in initial])
    final_arr = np.array([vec.value_in_unit(unit.nanometer) for vec in final])
    diff = final_arr - init
    return float(math.sqrt((diff**2).sum() / len(diff)))


def compute_radius_of_gyration(positions):
    arr = np.array([vec.value_in_unit(unit.nanometer) for vec in positions])
    center = arr.mean(axis=0)
    diff = arr - center
    return float(math.sqrt((diff**2).sum() / len(arr)))


if __name__ == "__main__":
    sys.exit(main())
