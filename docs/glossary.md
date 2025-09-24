# LogLine Folding Engine Glossary

- **Span**: Atomic record of a rotation event, including entropy/ information deltas and execution duration.
- **Contract**: Sequenced list of executable span instructions (e.g., rotate, clash_check) defined in `.lll` grammar.
- **Diamond**: Stable fold artifact produced after enforcement validates an optimized trajectory.
- **Traj / Trajectory**: Accumulated path of spans executed during a fold attempt; stores aggregate ΔS, Δi, and timing.
- **ΔS (Delta Entropy)**: Entropy change measured per span; used to track thermodynamic feasibility.
- **Δi (Delta Information)**: Information compression achieved per span; proxy for computational efficiency.
- **Ghost Mode**: Simulation modality where spans execute without committing to the canonical timeline.
- **Rollback**: Operation that reverts state to a prior span checkpoint after a rule violation.
- **LogLine ID**: Hash-based signature identifying the current fold state or recorded span.
- **Trust Index**: Computable confidence score derived from enforcement history and provenance checks.
