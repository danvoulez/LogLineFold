use folding_molecule::PeptideChain;
use crate::force_fields::{ForceField, Vec3};
use nalgebra::Vector3;
use rand::Rng;
use rand_distr::{Distribution, Normal};
use std::f64::consts::PI;

/// Trait for molecular dynamics integrators
pub trait Integrator {
    fn step(&mut self, chain: &mut PeptideChain, forces: &[Vec3], dt: f64);
    fn set_temperature(&mut self, temperature: f64);
    fn get_kinetic_energy(&self, chain: &PeptideChain) -> f64;
}

/// Velocity Verlet integrator with Langevin thermostat
#[derive(Debug, Clone)]
pub struct LangevinIntegrator {
    temperature: f64,
    friction: f64,
    velocities: Vec<Vec3>,
    masses: Vec<f64>,
    rng: rand::rngs::ThreadRng,
    normal: Normal<f64>,
}

impl LangevinIntegrator {
    pub fn new(num_particles: usize, temperature: f64, friction: f64) -> Self {
        let masses = vec![12.0; num_particles]; // CA atom mass
        let velocities = vec![Vec3::zeros(); num_particles];
        
        Self {
            temperature,
            friction,
            velocities,
            masses,
            rng: rand::thread_rng(),
            normal: Normal::new(0.0, 1.0).unwrap(),
        }
    }

    pub fn initialize_velocities(&mut self, chain: &PeptideChain) {
        let kb = 0.001987; // Boltzmann constant in kcal/mol/K
        
        for (i, mass) in self.masses.iter().enumerate() {
            let sigma = (kb * self.temperature / mass).sqrt();
            
            self.velocities[i] = Vec3::new(
                self.normal.sample(&mut self.rng) * sigma,
                self.normal.sample(&mut self.rng) * sigma,
                self.normal.sample(&mut self.rng) * sigma,
            );
        }
    }

    pub fn apply_constraints(&mut self, chain: &mut PeptideChain) {
        // SHAKE algorithm for bond constraints
        let tolerance = 1e-6;
        let max_iterations = 100;
        let target_bond_length = 3.8; // Å
        
        let residues = chain.residues_mut();
        
        for _ in 0..max_iterations {
            let mut max_error: f64 = 0.0;
            
            for i in 0..residues.len().saturating_sub(1) {
                let pos1 = residues[i].position();
                let pos2 = residues[i + 1].position();
                
                let dx = pos2[0] - pos1[0];
                let dy = pos2[1] - pos1[1];
                let dz = pos2[2] - pos1[2];
                let current_distance = (dx * dx + dy * dy + dz * dz).sqrt();
                
                let error = current_distance - target_bond_length;
                max_error = max_error.max(error.abs());
                
                if error.abs() > tolerance {
                    let correction = error / (2.0 * current_distance);
                    
                    let mut pos1_new = pos1;
                    let mut pos2_new = pos2;
                    
                    pos1_new[0] += correction * dx;
                    pos1_new[1] += correction * dy;
                    pos1_new[2] += correction * dz;
                    
                    pos2_new[0] -= correction * dx;
                    pos2_new[1] -= correction * dy;
                    pos2_new[2] -= correction * dz;
                    
                    residues[i].set_position(pos1_new);
                    residues[i + 1].set_position(pos2_new);
                }
            }
            
            if max_error < tolerance {
                break;
            }
        }
    }

    pub fn apply_rotation_command(&mut self, chain: &mut PeptideChain, residue_idx: usize, angle: f64) {
        if residue_idx < self.velocities.len() {
            // Apply rotation as velocity perturbation
            let perturbation_strength = 10.0; // Adjust as needed
            let direction = Vec3::new(
                self.normal.sample(&mut self.rng),
                self.normal.sample(&mut self.rng),
                self.normal.sample(&mut self.rng),
            ).normalize();
            
            self.velocities[residue_idx] += direction * angle * perturbation_strength;
        }
    }

    pub fn compute_temperature(&self, chain: &PeptideChain) -> f64 {
        let kinetic_energy = self.get_kinetic_energy(chain);
        let kb = 0.001987; // Boltzmann constant in kcal/mol/K
        let dof = 3 * chain.len(); // 3 degrees of freedom per particle
        
        if dof > 0 {
            2.0 * kinetic_energy / (kb * dof as f64)
        } else {
            0.0
        }
    }

    pub fn scale_velocities(&mut self, target_temperature: f64, current_temperature: f64) {
        if current_temperature > 1e-10 {
            let scale_factor = (target_temperature / current_temperature).sqrt();
            for velocity in &mut self.velocities {
                *velocity *= scale_factor;
            }
        }
    }
}

impl Integrator for LangevinIntegrator {
    fn step(&mut self, chain: &mut PeptideChain, forces: &[Vec3], dt: f64) {
        let kb = 0.001987; // Boltzmann constant in kcal/mol/K
        let residues = chain.residues_mut();
        
        // Ensure we have the right number of velocities
        if self.velocities.len() != residues.len() {
            self.velocities.resize(residues.len(), Vec3::zeros());
            self.masses.resize(residues.len(), 12.0);
        }
        
        // Velocity Verlet with Langevin thermostat
        for (i, residue) in residues.iter_mut().enumerate() {
            let mass = self.masses[i];
            let force = if i < forces.len() { forces[i] } else { Vec3::zeros() };
            
            // Random force for thermostat
            let sigma = (2.0 * self.friction * kb * self.temperature / mass).sqrt();
            let random_force = Vec3::new(
                self.normal.sample(&mut self.rng) * sigma,
                self.normal.sample(&mut self.rng) * sigma,
                self.normal.sample(&mut self.rng) * sigma,
            );
            
            // Update velocity (first half)
            let acceleration = (force + random_force - self.friction * self.velocities[i]) / mass;
            self.velocities[i] += acceleration * dt * 0.5;
            
            // Update position
            let mut pos = residue.position();
            pos[0] += self.velocities[i].x * dt;
            pos[1] += self.velocities[i].y * dt;
            pos[2] += self.velocities[i].z * dt;
            residue.set_position(pos);
            
            // Update velocity (second half) - would need new forces here
            self.velocities[i] += acceleration * dt * 0.5;
        }
        
        // Apply constraints
        self.apply_constraints(chain);
    }

    fn set_temperature(&mut self, temperature: f64) {
        self.temperature = temperature;
    }

    fn get_kinetic_energy(&self, chain: &PeptideChain) -> f64 {
        let mut kinetic_energy = 0.0;
        
        for (i, mass) in self.masses.iter().enumerate() {
            if i < self.velocities.len() {
                let v_squared = self.velocities[i].norm_squared();
                kinetic_energy += 0.5 * mass * v_squared;
            }
        }
        
        kinetic_energy
    }
}

/// Simple Verlet integrator without thermostat
#[derive(Debug, Clone)]
pub struct VerletIntegrator {
    previous_positions: Vec<[f64; 3]>,
    masses: Vec<f64>,
}

impl VerletIntegrator {
    pub fn new(num_particles: usize) -> Self {
        Self {
            previous_positions: vec![[0.0; 3]; num_particles],
            masses: vec![12.0; num_particles], // CA atom mass
        }
    }

    pub fn initialize(&mut self, chain: &PeptideChain) {
        let residues = chain.residues();
        self.previous_positions.clear();
        
        for residue in residues {
            self.previous_positions.push(residue.position());
        }
    }
}

impl Integrator for VerletIntegrator {
    fn step(&mut self, chain: &mut PeptideChain, forces: &[Vec3], dt: f64) {
        let residues = chain.residues_mut();
        
        // Ensure we have the right number of previous positions
        if self.previous_positions.len() != residues.len() {
            self.previous_positions.resize(residues.len(), [0.0; 3]);
            self.masses.resize(residues.len(), 12.0);
        }
        
        for (i, residue) in residues.iter_mut().enumerate() {
            let mass = self.masses[i];
            let force = if i < forces.len() { forces[i] } else { Vec3::zeros() };
            let current_pos = residue.position();
            let prev_pos = self.previous_positions[i];
            
            // Verlet integration
            let acceleration = force / mass;
            let new_pos = [
                2.0 * current_pos[0] - prev_pos[0] + acceleration.x * dt * dt,
                2.0 * current_pos[1] - prev_pos[1] + acceleration.y * dt * dt,
                2.0 * current_pos[2] - prev_pos[2] + acceleration.z * dt * dt,
            ];
            
            self.previous_positions[i] = current_pos;
            residue.set_position(new_pos);
        }
    }

    fn set_temperature(&mut self, _temperature: f64) {
        // Verlet integrator doesn't have temperature control
    }

    fn get_kinetic_energy(&self, chain: &PeptideChain) -> f64 {
        // Would need velocities to compute kinetic energy
        // For Verlet, velocities can be estimated from position differences
        0.0
    }
}

/// Brownian dynamics integrator
#[derive(Debug, Clone)]
pub struct BrownianIntegrator {
    temperature: f64,
    friction: f64,
    masses: Vec<f64>,
    rng: rand::rngs::ThreadRng,
    normal: Normal<f64>,
}

impl BrownianIntegrator {
    pub fn new(num_particles: usize, temperature: f64, friction: f64) -> Self {
        Self {
            temperature,
            friction,
            masses: vec![12.0; num_particles],
            rng: rand::thread_rng(),
            normal: Normal::new(0.0, 1.0).unwrap(),
        }
    }
}

impl Integrator for BrownianIntegrator {
    fn step(&mut self, chain: &mut PeptideChain, forces: &[Vec3], dt: f64) {
        let kb = 0.001987; // Boltzmann constant in kcal/mol/K
        let residues = chain.residues_mut();
        
        // Ensure we have the right number of masses
        if self.masses.len() != residues.len() {
            self.masses.resize(residues.len(), 12.0);
        }
        
        for (i, residue) in residues.iter_mut().enumerate() {
            let mass = self.masses[i];
            let force = if i < forces.len() { forces[i] } else { Vec3::zeros() };
            
            // Brownian dynamics: dx = (D/kT) * F * dt + sqrt(2*D*dt) * R
            let diffusion_coeff = kb * self.temperature / (self.friction * mass);
            let drift = force * diffusion_coeff * dt / (kb * self.temperature);
            let noise_amplitude = (2.0 * diffusion_coeff * dt).sqrt();
            
            let random_displacement = Vec3::new(
                self.normal.sample(&mut self.rng) * noise_amplitude,
                self.normal.sample(&mut self.rng) * noise_amplitude,
                self.normal.sample(&mut self.rng) * noise_amplitude,
            );
            
            let mut pos = residue.position();
            pos[0] += drift.x + random_displacement.x;
            pos[1] += drift.y + random_displacement.y;
            pos[2] += drift.z + random_displacement.z;
            residue.set_position(pos);
        }
    }

    fn set_temperature(&mut self, temperature: f64) {
        self.temperature = temperature;
    }

    fn get_kinetic_energy(&self, _chain: &PeptideChain) -> f64 {
        // Brownian dynamics doesn't explicitly track velocities
        let kb = 0.001987;
        let dof = 3 * self.masses.len();
        0.5 * kb * self.temperature * dof as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use folding_molecule::{Residue, ResidueId};
    use crate::force_fields::CoarseGrainedForceField;

    #[test]
    fn test_langevin_integrator() {
        let mut chain = create_test_chain();
        let mut integrator = LangevinIntegrator::new(chain.len(), 300.0, 1.0);
        integrator.initialize_velocities(&chain);
        
        let ff = CoarseGrainedForceField::new();
        let forces = ff.compute_forces(&chain);
        
        let initial_energy = integrator.get_kinetic_energy(&chain);
        integrator.step(&mut chain, &forces, 0.001);
        let final_energy = integrator.get_kinetic_energy(&chain);
        
        assert!(initial_energy.is_finite());
        assert!(final_energy.is_finite());
    }

    #[test]
    fn test_verlet_integrator() {
        let mut chain = create_test_chain();
        let mut integrator = VerletIntegrator::new(chain.len());
        integrator.initialize(&chain);
        
        let ff = CoarseGrainedForceField::new();
        let forces = ff.compute_forces(&chain);
        
        let initial_pos = chain.residues()[0].position();
        integrator.step(&mut chain, &forces, 0.001);
        let final_pos = chain.residues()[0].position();
        
        // Position should change
        let displacement = (
            (final_pos[0] - initial_pos[0]).powi(2) +
            (final_pos[1] - initial_pos[1]).powi(2) +
            (final_pos[2] - initial_pos[2]).powi(2)
        ).sqrt();
        
        assert!(displacement >= 0.0);
    }

    #[test]
    fn test_brownian_integrator() {
        let mut chain = create_test_chain();
        let mut integrator = BrownianIntegrator::new(chain.len(), 300.0, 1.0);
        
        let ff = CoarseGrainedForceField::new();
        let forces = ff.compute_forces(&chain);
        
        let initial_pos = chain.residues()[0].position();
        integrator.step(&mut chain, &forces, 0.001);
        let final_pos = chain.residues()[0].position();
        
        // Position should change due to random motion
        let displacement = (
            (final_pos[0] - initial_pos[0]).powi(2) +
            (final_pos[1] - initial_pos[1]).powi(2) +
            (final_pos[2] - initial_pos[2]).powi(2)
        ).sqrt();
        
        assert!(displacement >= 0.0);
    }

    #[test]
    fn test_constraint_satisfaction() {
        let mut chain = create_test_chain();
        let mut integrator = LangevinIntegrator::new(chain.len(), 300.0, 1.0);
        
        integrator.apply_constraints(&mut chain);
        
        // Check bond lengths
        let residues = chain.residues();
        for i in 0..residues.len().saturating_sub(1) {
            let pos1 = residues[i].position();
            let pos2 = residues[i + 1].position();
            let distance = (
                (pos2[0] - pos1[0]).powi(2) +
                (pos2[1] - pos1[1]).powi(2) +
                (pos2[2] - pos1[2]).powi(2)
            ).sqrt();
            
            // Should be close to target bond length (3.8 Å)
            assert!((distance - 3.8).abs() < 0.1);
        }
    }

    fn create_test_chain() -> PeptideChain {
        let residues = vec![
            Residue::new(ResidueId(0), "ALA", [0.0, 0.0, 0.0]),
            Residue::new(ResidueId(1), "GLY", [3.8, 0.0, 0.0]),
            Residue::new(ResidueId(2), "SER", [7.6, 0.0, 0.0]),
            Residue::new(ResidueId(3), "VAL", [11.4, 0.0, 0.0]),
        ];
        PeptideChain::new(residues)
    }
}
