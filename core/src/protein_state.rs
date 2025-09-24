use folding_molecule::{EnergyModel, PeptideChain, ResidueId};
use folding_time::Trajectory;

/// Runtime representation of the protein being folded.
#[derive(Debug)]
pub struct ProteinState {
    pub chain: PeptideChain,
    pub trajectory: Trajectory,
    pub energy_model: EnergyModel,
}

/// Snapshot used for commit/rollback operations.
#[derive(Clone, Debug)]
pub struct ProteinSnapshot {
    pub chain: PeptideChain,
    pub trajectory: Trajectory,
}

/// Aggregated energy measurements for the current fold attempt.
#[derive(Debug, Default, Clone)]
pub struct EnergyState {
    pub total_potential: f64,
    pub total_kinetic: f64,
}

impl ProteinState {
    pub fn new(chain: PeptideChain, energy_model: EnergyModel) -> Self {
        Self {
            chain,
            trajectory: Trajectory::new(),
            energy_model,
        }
    }

    pub fn trajectory(&self) -> &Trajectory {
        &self.trajectory
    }

    pub fn trajectory_mut(&mut self) -> &mut Trajectory {
        &mut self.trajectory
    }

    pub fn energy_state(&self) -> EnergyState {
        let summary = self.energy_model.energy_summary(&self.chain);
        let kinetic = self
            .chain
            .residues()
            .iter()
            .map(|residue| residue.phi.abs() + residue.psi.abs())
            .sum();
        EnergyState {
            total_potential: summary.total(),
            total_kinetic: kinetic,
        }
    }

    pub fn apply_rotation(&mut self, residue: ResidueId, delta_angle: f64) {
        if let Some(residue_entry) = self.chain.residue_mut(residue) {
            residue_entry.phi += delta_angle;
            let current_position = residue_entry.position();
            let radius = (current_position[0].powi(2) + current_position[1].powi(2))
                .sqrt()
                .max(1.0);
            let angle_rad = residue_entry.phi.to_radians();
            let new_position = [
                radius * angle_rad.cos(),
                radius * angle_rad.sin(),
                current_position[2],
            ];
            residue_entry.set_position(new_position);
        }
    }

    pub fn snapshot(&self) -> ProteinSnapshot {
        ProteinSnapshot {
            chain: self.chain.clone(),
            trajectory: self.trajectory.clone(),
        }
    }

    pub fn restore(&mut self, snapshot: ProteinSnapshot) {
        self.chain = snapshot.chain;
        self.trajectory = snapshot.trajectory;
    }
}
