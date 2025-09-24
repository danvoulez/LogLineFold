# `.lll` Contract Specification (Draft)

## File Structure
- UTF-8 plaintext.
- One instruction per line.
- `#` begins a comment that runs to end of line.

## Instructions

### `rotate <residue_id> <angle_degrees> [duration_ms]`
- `residue_id`: zero-based integer referencing `ResidueId` in the active chain.
- `angle_degrees`: signed floating point degrees applied to the residue torsion.
- `duration_ms`: optional integer; defaults to `1` if omitted.

Example:
```
rotate 3 45.0 5
```

### `clash_check`
Trigger spatial validation for the current partial rotation buffer.

### `commit`
Persist current spans and energy snapshot to the timeline, producing a signed log entry.

### `rollback`
Revert state to the last committed span, discarding uncommitted rotations.

### `define_domain <[name]> <start>-<end>`
Declare a logical folding unit. The optional `name` is associated with the residue range identified by `<start>-<end>` (inclusive, zero-based indexes).

Examples:
```
define_domain helixA 5-20
define_domain 35-48
```

### `require_chaperone <chaperone> [for <span_label>]`
Mark that subsequent spans require the presence of a helper (e.g., Hsp70). Optionally reference a previously defined `span_alias`.

Example:
```
require_chaperone Hsp70 for helixA
```

### `add_modification <modification> at <residue>`
Annotate the sequence with a post-translational modification. The residue may be referenced as an index or residue code + index (e.g., `S50`).

Example:
```
add_modification phosphorylation at S50
```

### `set_physics_level <level>`
Switch the physics backend. Accepted values: `toy`, `coarse`, `gb`, `full`. Contracts may promote certain spans to implicit/explicit solvent or coarse-grained execution.

Example:
```
set_physics_level GB
```

### `physics_span <on|off>`
Toggle physics-backed execution for subsequent `rotate` spans. When `on`, the runtime delegates to the OpenMM bridge where available; otherwise it falls back to the toy solver.

Example:
```
physics_span on
```

## Validation Rules (to be enforced)
- Rotation magnitude must respect `Ruleset::max_rotation_degrees`.
- Bond constraints defined in `BondConstraintSet` must remain satisfied.
- Entropy and information budgets must stay within configured thresholds.

## Extensions (planned)
- `span_alias <name>`: assign human-readable label to upcoming spans.
- `ghost on|off`: toggle ghost mode execution.
- `load_preset <name>`: import contract fragments.
