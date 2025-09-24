use folding_molecule::PeptideChain;
use crate::force_fields::{ForceField, CoarseGrainedForceField, Amber99SBForceField};
use crate::integrators::{Integrator, LangevinIntegrator, VerletIntegrator};
use crate::PhysicsLevel;
use serde_json;
use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicsRequest {
    pub initial_positions: Vec<[f64; 3]>,
    pub residue_types: Vec<String>,
    pub rotation_commands: Vec<(usize, f64)>,
    pub physics_level: PhysicsLevel,
    pub temperature: f64,
    pub simulation_time: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RotationOutcome {
    pub final_positions: Vec<[f64; 3]>,
    pub final_angles: Vec<(f64, f64)>,
    pub energy: f64,
    pub kinetic_energy: f64,
    pub potential_energy: f64,
    pub temperature: f64,
    pub rmsd: f64,
    pub radius_of_gyration: f64,
    pub simulation_time: f64,
    pub convergence_info: String,
    pub trajectory_data: Option<serde_json::Value>,
}

/// Native Rust physics engine bridge
pub struct NativePhysicsBridge {
    force_field: Box<dyn ForceField>,
    integrator: Box<dyn Integrator>,
    physics_level: PhysicsLevel,
}

impl NativePhysicsBridge {
    pub fn new(physics_level: PhysicsLevel) -> Self {
        let (force_field, integrator): (Box<dyn ForceField>, Box<dyn Integrator>) = match physics_level {
            PhysicsLevel::Toy => {
                let ff = Box::new(CoarseGrainedForceField::new());
                let integrator = Box::new(VerletIntegrator::new(0)); // Will be resized
                (ff, integrator)
            },
            PhysicsLevel::Coarse => {
                let ff = Box::new(CoarseGrainedForceField::new());
                let integrator = Box::new(LangevinIntegrator::new(0, 300.0, 1.0));
                (ff, integrator)
            },
            PhysicsLevel::GB => {
                let ff = Box::new(Amber99SBForceField::new());
                let integrator = Box::new(LangevinIntegrator::new(0, 300.0, 5.0));
                (ff, integrator)
            },
            PhysicsLevel::Full => {
                let ff = Box::new(Amber99SBForceField::new());
                let integrator = Box::new(LangevinIntegrator::new(0, 300.0, 10.0));
                (ff, integrator)
            },
        };

        Self {
            force_field,
            integrator,
            physics_level,
        }
    }

    pub fn run_physics_simulation(&mut self, request: &PhysicsRequest) -> Result<RotationOutcome, String> {
        let start_time = Instant::now();
        
        // Parse the chain from request
        let mut chain = self.parse_chain_from_request(request)?;
        
        // Set up integrator parameters based on physics level
        let (timestep, num_steps, temperature) = self.get_simulation_parameters();
        self.integrator.set_temperature(temperature);
        
        // Initialize integrator if it's Langevin (simplified approach)
        // Apply rotation commands by modifying phi/psi angles directly
        for (residue_idx, angle) in &request.rotation_commands {
            if *residue_idx < chain.len() {
                let residues = chain.residues_mut();
                if let Some(residue) = residues.get_mut(*residue_idx) {
                    residue.phi += angle;
                }
            }
        }
        
        // Run MD simulation
        let mut energies = Vec::new();
        let mut temperatures = Vec::new();
        
        for step in 0..num_steps {
            // Compute forces
            let forces = self.force_field.compute_forces(&chain);
            
            // Integrate one step
            self.integrator.step(&mut chain, &forces, timestep);
            
            // Record diagnostics every 10 steps
            if step % 10 == 0 {
                use crate::force_fields::ForceField;
                use crate::integrators::Integrator;
                let potential_energy = self.force_field.compute_energy(&chain);
                let kinetic_energy = self.integrator.get_kinetic_energy(&chain);
                let total_energy = potential_energy + kinetic_energy;
                
                energies.push(total_energy);
                temperatures.push(temperature);
            }
        }
        
        // Compute final metrics
        use crate::force_fields::ForceField;
        use crate::integrators::Integrator;
        let final_energy = self.force_field.compute_energy(&chain);
        let kinetic_energy = self.integrator.get_kinetic_energy(&chain);
        let rmsd = self.compute_rmsd(&chain, &request.initial_positions);
        let radius_of_gyration = self.compute_radius_of_gyration(&chain);
        
        let simulation_time = start_time.elapsed().as_secs_f64();
        
        Ok(RotationOutcome {
            final_positions: self.extract_positions(&chain),
            final_angles: self.extract_angles(&chain),
            energy: final_energy,
            kinetic_energy,
            potential_energy: final_energy - kinetic_energy,
            temperature: temperatures.last().copied().unwrap_or(temperature),
            rmsd,
            radius_of_gyration,
            simulation_time,
            convergence_info: format!(
                "Native physics simulation completed in {:.3}s with {} steps",
                simulation_time, num_steps
            ),
            trajectory_data: Some(serde_json::json!({
                "energies": energies,
                "temperatures": temperatures,
                "physics_level": format!("{:?}", self.physics_level),
                "timestep": timestep,
                "num_steps": num_steps
            })),
        })
    }
    
    fn get_simulation_parameters(&self) -> (f64, usize, f64) {
        match self.physics_level {
            PhysicsLevel::Toy => (0.01, 100, 300.0),      // 1 ps total
            PhysicsLevel::Coarse => (0.005, 200, 300.0),  // 1 ps total
            PhysicsLevel::GB => (0.002, 500, 300.0),      // 1 ps total
            PhysicsLevel::Full => (0.001, 1000, 300.0),   // 1 ps total
        }
    }
    
    fn parse_chain_from_request(&self, request: &PhysicsRequest) -> Result<PeptideChain, String> {
        use folding_molecule::{Residue, ResidueId};
        
        if request.initial_positions.len() != request.residue_types.len() {
            return Err("Mismatch between positions and residue types".to_string());
        }
        
        let residues: Vec<Residue> = request.initial_positions
            .iter()
            .zip(request.residue_types.iter())
            .enumerate()
            .map(|(i, (pos, res_type))| {
                Residue::new(ResidueId(i), res_type, *pos)
            })
            .collect();
        
        Ok(PeptideChain::new(residues))
    }
    
    fn extract_positions(&self, chain: &PeptideChain) -> Vec<[f64; 3]> {
        chain.residues().iter().map(|r| r.position()).collect()
    }
    
    fn extract_angles(&self, chain: &PeptideChain) -> Vec<(f64, f64)> {
        chain.residues().iter().map(|r| (r.phi, r.psi)).collect()
    }
    
    fn compute_rmsd(&self, chain: &PeptideChain, reference_positions: &[[f64; 3]]) -> f64 {
        let current_positions = self.extract_positions(chain);
        
        if current_positions.len() != reference_positions.len() {
            return 0.0;
        }
        
        let mut sum_squared_deviations = 0.0;
        for (current, reference) in current_positions.iter().zip(reference_positions.iter()) {
            let dx = current[0] - reference[0];
            let dy = current[1] - reference[1];
            let dz = current[2] - reference[2];
            sum_squared_deviations += dx * dx + dy * dy + dz * dz;
        }
        
        (sum_squared_deviations / current_positions.len() as f64).sqrt()
    }
    
    fn compute_radius_of_gyration(&self, chain: &PeptideChain) -> f64 {
        let positions = self.extract_positions(chain);
        let n = positions.len() as f64;
        
        if n == 0.0 {
            return 0.0;
        }
        
        // Compute center of mass
        let mut com = [0.0; 3];
        for pos in &positions {
            com[0] += pos[0];
            com[1] += pos[1];
            com[2] += pos[2];
        }
        com[0] /= n;
        com[1] /= n;
        com[2] /= n;
        
        // Compute radius of gyration
        let mut rg_squared = 0.0;
        for pos in &positions {
            let dx = pos[0] - com[0];
            let dy = pos[1] - com[1];
            let dz = pos[2] - com[2];
            rg_squared += dx * dx + dy * dy + dz * dz;
        }
        
        (rg_squared / n).sqrt()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_bridge::PhysicsRequest;

    #[test]
    fn test_native_bridge_creation() {
        let bridge = NativePhysicsBridge::new(PhysicsLevel::Coarse);
        assert!(matches!(bridge.physics_level, PhysicsLevel::Coarse));
    }

    #[test]
    fn test_physics_simulation() {
        let mut bridge = NativePhysicsBridge::new(PhysicsLevel::Toy);
        
        let request = PhysicsRequest {
            initial_positions: vec![
                [0.0, 0.0, 0.0],
                [3.8, 0.0, 0.0],
                [7.6, 0.0, 0.0],
                [11.4, 0.0, 0.0],
            ],
            residue_types: vec!["ALA".to_string(), "GLY".to_string(), "SER".to_string(), "VAL".to_string()],
            rotation_commands: vec![(1, 0.1), (2, -0.1)],
            physics_level: PhysicsLevel::Toy,
            temperature: 300.0,
            simulation_time: 1.0,
        };
        
        let result = bridge.run_physics_simulation(&request);
        assert!(result.is_ok());
        
        let outcome = result.unwrap();
        assert_eq!(outcome.final_positions.len(), 4);
        assert_eq!(outcome.final_angles.len(), 4);
        assert!(outcome.energy.is_finite());
        assert!(outcome.rmsd >= 0.0);
        assert!(outcome.radius_of_gyration >= 0.0);
        assert!(outcome.simulation_time > 0.0);
    }

    #[test]
    fn test_rmsd_calculation() {
        let bridge = NativePhysicsBridge::new(PhysicsLevel::Toy);
        
        use folding_molecule::{PeptideChain, Residue, ResidueId};
        let residues = vec![
            Residue::new(ResidueId(0), "ALA", [0.0, 0.0, 0.0]),
            Residue::new(ResidueId(1), "GLY", [3.8, 0.0, 0.0]),
        ];
        let chain = PeptideChain::new(residues);
        
        let reference = vec![[0.0, 0.0, 0.0], [3.8, 0.0, 0.0]];
        let rmsd = bridge.compute_rmsd(&chain, &reference);
        
        assert!((rmsd - 0.0).abs() < 1e-10); // Should be zero for identical structures
    }

    #[test]
    fn test_radius_of_gyration() {
        let bridge = NativePhysicsBridge::new(PhysicsLevel::Toy);
        
        use folding_molecule::{PeptideChain, Residue, ResidueId};
        let residues = vec![
            Residue::new(ResidueId(0), "ALA", [0.0, 0.0, 0.0]),
            Residue::new(ResidueId(1), "GLY", [3.8, 0.0, 0.0]),
        ];
        let chain = PeptideChain::new(residues);
        
        let rg = bridge.compute_radius_of_gyration(&chain);
        assert!(rg > 0.0);
        assert!(rg.is_finite());
    }
}
