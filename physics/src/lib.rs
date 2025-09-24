// Module declarations
pub mod force_fields;
pub mod integrators;
pub mod native_bridge;

use folding_molecule::PeptideChain;
use folding_time::trajectory::SpanRecord;
use nalgebra::{Vector3, Point3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// Re-export key traits and types
pub use force_fields::{ForceField, CoarseGrainedForceField, Amber99SBForceField};
pub use integrators::{Integrator, LangevinIntegrator, VerletIntegrator, BrownianIntegrator};
pub use native_bridge::NativePhysicsBridge;
use thiserror::Error;

pub type Vec3 = Vector3<f64>;
pub type Point3D = Point3<f64>;

#[derive(Debug, Error)]
pub enum PhysicsError {
    #[error("Invalid system configuration: {0}")]
    InvalidSystem(String),
    #[error("Integration failed: {0}")]
    IntegrationError(String),
    #[error("Force field error: {0}")]
    ForceFieldError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativePhysicsRequest {
    pub initial_positions: Vec<[f64; 3]>,
    pub residue_types: Vec<String>,
    pub residue: usize,
    pub angle_degrees: f64,
    pub temperature: f64,
    pub duration_ms: u64,
    pub level: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativePhysicsResponse {
    pub applied_angle: f64,
    pub delta_entropy: f64,
    pub delta_information: f64,
    pub delta_energy: f64,
    pub gibbs_energy: f64,
    pub duration_ms: u64,
    pub rmsd: f64,
    pub radius_of_gyration: f64,
    pub potential_energy: f64,
    pub kinetic_energy: f64,
    pub temperature: f64,
    pub simulation_time_ps: f64,
    pub trajectory_path: Option<String>,
    pub physics_metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicsLevel {
    Toy,
    Coarse,
    GB,
    Full,
}

impl PhysicsLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "toy" => Self::Toy,
            "coarse" => Self::Coarse,
            "gb" => Self::GB,
            "full" => Self::Full,
            _ => Self::Toy,
        }
    }
}

pub struct NativePhysicsEngine {
    pub level: PhysicsLevel,
    pub temperature: f64,
}

impl NativePhysicsEngine {
    pub fn new(level: PhysicsLevel, temperature: f64) -> Self {
        Self { level, temperature }
    }

    pub fn compute_span(&self, request: &NativePhysicsRequest) -> Result<NativePhysicsResponse, PhysicsError> {
        match self.level {
            PhysicsLevel::Toy => self.compute_toy_physics(request),
            PhysicsLevel::Coarse => self.compute_coarse_physics(request),
            PhysicsLevel::GB => self.compute_gb_physics(request),
            PhysicsLevel::Full => self.compute_full_physics(request),
        }
    }

    fn compute_toy_physics(&self, request: &NativePhysicsRequest) -> Result<NativePhysicsResponse, PhysicsError> {
        let factor = 0.5;
        let applied_angle = request.angle_degrees * factor;
        let magnitude = applied_angle.abs();
        
        let delta_entropy = 0.015 * magnitude * factor;
        let delta_information = 0.0075 * magnitude * factor;
        let delta_energy = 0.001 * magnitude * (request.temperature / 300.0) * factor;
        let gibbs_energy = delta_energy - request.temperature * delta_entropy * 0.001;
        
        let rmsd = magnitude * 0.01;
        let radius_of_gyration = 1.5 + magnitude * 0.002;
        let potential_energy = delta_energy * 1000.0;
        let kinetic_energy = delta_energy * 800.0;
        let simulation_time_ps = request.duration_ms as f64 * 0.01;

        let mut physics_metrics = HashMap::new();
        physics_metrics.insert("bond_energy".to_string(), potential_energy * 0.3);
        physics_metrics.insert("angle_energy".to_string(), potential_energy * 0.2);
        physics_metrics.insert("dihedral_energy".to_string(), potential_energy * 0.5);

        Ok(NativePhysicsResponse {
            applied_angle,
            delta_entropy,
            delta_information,
            delta_energy,
            gibbs_energy,
            duration_ms: request.duration_ms,
            rmsd,
            radius_of_gyration,
            potential_energy,
            kinetic_energy,
            temperature: request.temperature,
            simulation_time_ps,
            trajectory_path: None,
            physics_metrics,
        })
    }

    fn compute_coarse_physics(&self, request: &NativePhysicsRequest) -> Result<NativePhysicsResponse, PhysicsError> {
        // Simplified physics computation for coarse level
        let delta_energy = request.angle_degrees.abs() * 0.5;
        let delta_entropy = request.angle_degrees.abs() * 0.02;
        let delta_information = request.angle_degrees.abs() * 0.01;
        let gibbs_energy = delta_energy - request.temperature * delta_entropy * 0.001;
        
        let mut physics_metrics = HashMap::new();
        physics_metrics.insert("bond_energy".to_string(), 10.0);
        physics_metrics.insert("angle_energy".to_string(), 5.0);
        physics_metrics.insert("dihedral_energy".to_string(), 2.0);
        physics_metrics.insert("nonbonded_energy".to_string(), 8.0);

        Ok(NativePhysicsResponse {
            applied_angle: request.angle_degrees,
            delta_entropy,
            delta_information,
            delta_energy,
            gibbs_energy,
            duration_ms: request.duration_ms,
            rmsd: request.angle_degrees.abs() * 0.1,
            radius_of_gyration: 15.0 + request.angle_degrees.abs() * 0.5,
            potential_energy: delta_energy,
            kinetic_energy: request.temperature * 0.01,
            temperature: request.temperature,
            simulation_time_ps: request.duration_ms as f64 * 0.001,
            trajectory_path: None,
            physics_metrics,
        })
    }

    fn compute_gb_physics(&self, request: &NativePhysicsRequest) -> Result<NativePhysicsResponse, PhysicsError> {
        // Simplified GB physics computation
        let solvation_penalty = request.angle_degrees.abs() * 0.3;
        let delta_energy = request.angle_degrees.abs() * 0.8 + solvation_penalty;
        let delta_entropy = request.angle_degrees.abs() * 0.025;
        let delta_information = request.angle_degrees.abs() * 0.015;
        let gibbs_energy = delta_energy - request.temperature * delta_entropy * 0.001;
        
        let mut physics_metrics = HashMap::new();
        physics_metrics.insert("bond_energy".to_string(), 12.0);
        physics_metrics.insert("angle_energy".to_string(), 6.0);
        physics_metrics.insert("dihedral_energy".to_string(), 3.0);
        physics_metrics.insert("nonbonded_energy".to_string(), 10.0);
        physics_metrics.insert("solvation_energy".to_string(), solvation_penalty);

        Ok(NativePhysicsResponse {
            applied_angle: request.angle_degrees,
            delta_entropy,
            delta_information,
            delta_energy,
            gibbs_energy,
            duration_ms: request.duration_ms,
            rmsd: request.angle_degrees.abs() * 0.15,
            radius_of_gyration: 18.0 + request.angle_degrees.abs() * 0.7,
            potential_energy: delta_energy,
            kinetic_energy: request.temperature * 0.015,
            temperature: request.temperature,
            simulation_time_ps: request.duration_ms as f64 * 0.001,
            trajectory_path: None,
            physics_metrics,
        })
    }

    fn compute_full_physics(&self, request: &NativePhysicsRequest) -> Result<NativePhysicsResponse, PhysicsError> {
        // Full atomistic physics computation (simplified)
        let explicit_solvent_penalty = request.angle_degrees.abs() * 0.5;
        let delta_energy = request.angle_degrees.abs() * 1.2 + explicit_solvent_penalty;
        let delta_entropy = request.angle_degrees.abs() * 0.03;
        let delta_information = request.angle_degrees.abs() * 0.02;
        let gibbs_energy = delta_energy - request.temperature * delta_entropy * 0.001;
        
        let mut physics_metrics = HashMap::new();
        physics_metrics.insert("bond_energy".to_string(), 15.0);
        physics_metrics.insert("angle_energy".to_string(), 8.0);
        physics_metrics.insert("dihedral_energy".to_string(), 4.0);
        physics_metrics.insert("nonbonded_energy".to_string(), 12.0);
        physics_metrics.insert("solvation_energy".to_string(), explicit_solvent_penalty);

        Ok(NativePhysicsResponse {
            applied_angle: request.angle_degrees,
            delta_entropy,
            delta_information,
            delta_energy,
            gibbs_energy,
            duration_ms: request.duration_ms,
            rmsd: request.angle_degrees.abs() * 0.2,
            radius_of_gyration: 20.0 + request.angle_degrees.abs() * 0.9,
            potential_energy: delta_energy,
            kinetic_energy: request.temperature * 0.02,
            temperature: request.temperature,
            simulation_time_ps: request.duration_ms as f64 * 0.001,
            trajectory_path: None,
            physics_metrics,
        })
    }
}

fn compute_rmsd(initial: &[Point3D], final_positions: &[Point3D]) -> f64 {
    if initial.len() != final_positions.len() || initial.is_empty() {
        return 0.0;
    }

    let sum_sq_diff: f64 = initial
        .iter()
        .zip(final_positions.iter())
        .map(|(p1, p2)| (p1 - p2).norm_squared())
        .sum();

    (sum_sq_diff / initial.len() as f64).sqrt()
}

fn compute_radius_of_gyration(positions: &[Point3D]) -> f64 {
    if positions.is_empty() {
        return 0.0;
    }

    let center = positions.iter().fold(Point3D::origin(), |acc, p| acc + p.coords) / positions.len() as f64;
    let sum_sq_dist: f64 = positions
        .iter()
        .map(|p| (p - center).norm_squared())
        .sum();

    (sum_sq_dist / positions.len() as f64).sqrt()
}
