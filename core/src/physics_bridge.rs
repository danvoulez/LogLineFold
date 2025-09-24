use crate::folding_parser::PhysicsLevel;
use crate::rotation_solver::{RotationCommand, RotationOutcome};
use folding_molecule::PeptideChain;

/// Request passed to the physics backend bridge.
pub struct PhysicsRequest<'a> {
    pub chain: &'a PeptideChain,
    pub command: RotationCommand,
    pub level: PhysicsLevel,
    pub temperature: f64,
}

/// Diagnostics captured when a physics backend services a span.
#[derive(Clone, Debug)]
pub struct PhysicsSpanMetrics {
    pub rmsd: f64,
    pub radius_of_gyration: f64,
    pub potential_energy: f64,
    pub kinetic_energy: f64,
    pub temperature: f64,
    pub simulation_time_ps: f64,
    pub trajectory_path: Option<String>,
}

/// Attempt to execute a physics-backed step. Returns `None` when the OpenMM
/// bridge is not available or the request cannot be satisfied.
pub fn run_physics_step(request: PhysicsRequest<'_>) -> Option<RotationOutcome> {
    #[cfg(feature = "openmm")]
    {
        openmm_bridge::run(request)
    }
    #[cfg(not(feature = "openmm"))]
    {
        let _ = request;
        None
    }
}

#[cfg(feature = "openmm")]
mod openmm_bridge {
    use super::PhysicsRequest;
    use crate::rotation_solver::RotationOutcome;
    use folding_time::trajectory::SpanRecord;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    #[derive(Serialize)]
    struct SerializedResidue {
        index: usize,
        position: [f64; 3],
    }

    #[derive(Serialize)]
    struct SerializedCommand {
        residue: usize,
        angle_degrees: f64,
        duration_ms: u64,
        label: Option<String>,
    }

    #[derive(Serialize)]
    struct BridgeRequest {
        level: String,
        temperature: f64,
        residues: Vec<SerializedResidue>,
        command: SerializedCommand,
    }

    #[derive(Deserialize)]
    struct BridgeResponse {
        applied_angle: f64,
        delta_entropy: f64,
        delta_information: f64,
        delta_energy: Option<f64>,
        gibbs_energy: Option<f64>,
        duration_ms: Option<u64>,
        rmsd: Option<f64>,
        radius_of_gyration: Option<f64>,
        potential_energy: Option<f64>,
        kinetic_energy: Option<f64>,
        temperature: Option<f64>,
        simulation_time_ps: Option<f64>,
        trajectory_path: Option<String>,
    }

    pub fn run(request: PhysicsRequest<'_>) -> Option<RotationOutcome> {
        let python = std::env::var("PYTHON_OPENMM_BIN").unwrap_or_else(|_| "python3".to_string());
        let script_path = openmm_script_path();
        let label = request
            .command
            .label
            .clone()
            .unwrap_or_else(|| format!("residue-{}", request.command.residue.0));

        let residues: Vec<SerializedResidue> = request
            .chain
            .residues()
            .iter()
            .map(|res| SerializedResidue {
                index: res.id.0,
                position: res.position(),
            })
            .collect();

        let payload = BridgeRequest {
            level: format_level(request.level),
            temperature: request.temperature,
            residues,
            command: SerializedCommand {
                residue: request.command.residue.0,
                angle_degrees: request.command.angle_degrees,
                duration_ms: request.command.duration.as_millis() as u64,
                label: Some(label.clone()),
            },
        };

        let mut child = Command::new(python)
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        if let Some(stdin) = child.stdin.as_mut() {
            if serde_json::to_writer(stdin, &payload).is_err() {
                return None;
            }
        }

        let output = child.wait_with_output().ok()?;
        if !output.status.success() {
            return None;
        }

        let response: BridgeResponse = serde_json::from_slice(&output.stdout).ok()?;
        let duration_ms = response
            .duration_ms
            .unwrap_or_else(|| request.command.duration.as_millis() as u64);
        let mut span = SpanRecord::new(
            label,
            response.delta_entropy,
            response.delta_information,
            Duration::from_millis(duration_ms.max(1)),
        );
        span.delta_theta = response.applied_angle;
        span.delta_energy = response.delta_energy.unwrap_or(0.0);
        span.gibbs_energy = response.gibbs_energy.unwrap_or(0.0);

        Some(RotationOutcome {
            applied_angle: response.applied_angle,
            span_record: span,
            ghost: false,
            physics_metrics: Some(super::PhysicsSpanMetrics {
                rmsd: response.rmsd.unwrap_or_default(),
                radius_of_gyration: response.radius_of_gyration.unwrap_or_default(),
                potential_energy: response.potential_energy.unwrap_or_default(),
                kinetic_energy: response.kinetic_energy.unwrap_or_default(),
                temperature: response.temperature.unwrap_or(request.temperature),
                simulation_time_ps: response.simulation_time_ps.unwrap_or(0.0),
                trajectory_path: response.trajectory_path,
            }),
        })
    }

    fn format_level(level: super::PhysicsLevel) -> String {
        match level {
            super::PhysicsLevel::Toy => "toy",
            super::PhysicsLevel::Coarse => "coarse",
            super::PhysicsLevel::Gb => "gb",
            super::PhysicsLevel::Full => "full",
        }
        .to_string()
    }

    fn openmm_script_path() -> PathBuf {
        if let Ok(path) = std::env::var("OPENMM_BRIDGE_SCRIPT") {
            PathBuf::from(path)
        } else {
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            PathBuf::from(manifest_dir).join("../physics/openmm_bridge.py")
        }
    }
}
