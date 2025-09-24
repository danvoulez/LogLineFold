use folding_molecule::PeptideChain;
use nalgebra::{Vector3, Point3};
use std::collections::HashMap;
use std::f64::consts::PI;

pub type Vec3 = Vector3<f64>;
pub type Point3D = Point3<f64>;

/// Trait for force field implementations
pub trait ForceField {
    fn compute_energy(&self, chain: &PeptideChain) -> f64;
    fn compute_forces(&self, chain: &PeptideChain) -> Vec<Vec3>;
    fn bond_energy(&self, chain: &PeptideChain) -> f64;
    fn angle_energy(&self, chain: &PeptideChain) -> f64;
    fn dihedral_energy(&self, chain: &PeptideChain) -> f64;
    fn nonbonded_energy(&self, chain: &PeptideChain) -> f64;
}

/// Coarse-grained force field for fast simulations
#[derive(Debug, Clone)]
pub struct CoarseGrainedForceField {
    bond_strength: f64,
    angle_strength: f64,
    dihedral_strength: f64,
    lj_epsilon: f64,
    lj_sigma: f64,
}

impl CoarseGrainedForceField {
    pub fn new() -> Self {
        Self {
            bond_strength: 100.0,  // kcal/mol/Å²
            angle_strength: 50.0,   // kcal/mol/rad²
            dihedral_strength: 2.0, // kcal/mol
            lj_epsilon: 0.2,        // kcal/mol
            lj_sigma: 3.5,          // Å
        }
    }
}

impl Default for CoarseGrainedForceField {
    fn default() -> Self {
        Self::new()
    }
}

impl ForceField for CoarseGrainedForceField {
    fn compute_energy(&self, chain: &PeptideChain) -> f64 {
        self.bond_energy(chain) + 
        self.angle_energy(chain) + 
        self.dihedral_energy(chain) + 
        self.nonbonded_energy(chain)
    }

    fn compute_forces(&self, chain: &PeptideChain) -> Vec<Vec3> {
        let residues = chain.residues();
        let mut forces = vec![Vec3::zeros(); residues.len()];
        
        // Bond forces
        for i in 0..residues.len().saturating_sub(1) {
            let pos1 = residues[i].position();
            let pos2 = residues[i + 1].position();
            let r = distance(pos1, pos2);
            let r0 = 3.8; // Target bond length
            
            if r > 1e-10 {
                let force_mag = -self.bond_strength * (r - r0);
                let direction = [
                    (pos2[0] - pos1[0]) / r,
                    (pos2[1] - pos1[1]) / r,
                    (pos2[2] - pos1[2]) / r,
                ];
                
                forces[i] += Vec3::new(
                    force_mag * direction[0],
                    force_mag * direction[1],
                    force_mag * direction[2],
                );
                forces[i + 1] -= Vec3::new(
                    force_mag * direction[0],
                    force_mag * direction[1],
                    force_mag * direction[2],
                );
            }
        }
        
        // Lennard-Jones forces
        for i in 0..residues.len() {
            for j in (i + 2)..residues.len() { // Skip bonded neighbors
                let pos1 = residues[i].position();
                let pos2 = residues[j].position();
                let r = distance(pos1, pos2);
                
                if r > 1e-10 && r < 12.0 { // Cutoff at 12 Å
                    let sigma_r = self.lj_sigma / r;
                    let sigma_r6 = sigma_r.powi(6);
                    let sigma_r12 = sigma_r6 * sigma_r6;
                    
                    let force_mag = 24.0 * self.lj_epsilon * (2.0 * sigma_r12 - sigma_r6) / r;
                    let direction = [
                        (pos2[0] - pos1[0]) / r,
                        (pos2[1] - pos1[1]) / r,
                        (pos2[2] - pos1[2]) / r,
                    ];
                    
                    forces[i] += Vec3::new(
                        force_mag * direction[0],
                        force_mag * direction[1],
                        force_mag * direction[2],
                    );
                    forces[j] -= Vec3::new(
                        force_mag * direction[0],
                        force_mag * direction[1],
                        force_mag * direction[2],
                    );
                }
            }
        }
        
        forces
    }

    fn bond_energy(&self, chain: &PeptideChain) -> f64 {
        let residues = chain.residues();
        let mut energy = 0.0;
        
        for i in 0..residues.len().saturating_sub(1) {
            let pos1 = residues[i].position();
            let pos2 = residues[i + 1].position();
            let r = distance(pos1, pos2);
            let r0 = 3.8; // Target bond length
            let dr = r - r0;
            energy += 0.5 * self.bond_strength * dr * dr;
        }
        
        energy
    }

    fn angle_energy(&self, chain: &PeptideChain) -> f64 {
        let residues = chain.residues();
        let mut energy = 0.0;
        
        for i in 0..residues.len().saturating_sub(2) {
            let pos1 = residues[i].position();
            let pos2 = residues[i + 1].position();
            let pos3 = residues[i + 2].position();
            
            let v1 = [pos1[0] - pos2[0], pos1[1] - pos2[1], pos1[2] - pos2[2]];
            let v2 = [pos3[0] - pos2[0], pos3[1] - pos2[1], pos3[2] - pos2[2]];
            
            let dot = v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2];
            let norm1 = (v1[0] * v1[0] + v1[1] * v1[1] + v1[2] * v1[2]).sqrt();
            let norm2 = (v2[0] * v2[0] + v2[1] * v2[1] + v2[2] * v2[2]).sqrt();
            
            if norm1 > 1e-10 && norm2 > 1e-10 {
                let cos_theta = (dot / (norm1 * norm2)).clamp(-1.0, 1.0);
                let theta = cos_theta.acos();
                let theta0 = 120.0 * PI / 180.0; // Target angle
                let dtheta = theta - theta0;
                energy += 0.5 * self.angle_strength * dtheta * dtheta;
            }
        }
        
        energy
    }

    fn dihedral_energy(&self, chain: &PeptideChain) -> f64 {
        let residues = chain.residues();
        let mut energy = 0.0;
        
        for i in 0..residues.len().saturating_sub(3) {
            // Simple dihedral potential based on phi/psi angles
            let phi = residues[i + 1].phi;
            let psi = residues[i + 1].psi;
            
            // Ramachandran-like potential
            energy += self.dihedral_strength * (1.0 + (3.0 * phi).cos());
            energy += self.dihedral_strength * (1.0 + (psi).cos());
        }
        
        energy
    }

    fn nonbonded_energy(&self, chain: &PeptideChain) -> f64 {
        let residues = chain.residues();
        let mut energy = 0.0;
        
        for i in 0..residues.len() {
            for j in (i + 2)..residues.len() { // Skip bonded neighbors
                let pos1 = residues[i].position();
                let pos2 = residues[j].position();
                let r = distance(pos1, pos2);
                
                if r > 1e-10 && r < 12.0 { // Cutoff at 12 Å
                    let sigma_r = self.lj_sigma / r;
                    let sigma_r6 = sigma_r.powi(6);
                    let sigma_r12 = sigma_r6 * sigma_r6;
                    
                    energy += 4.0 * self.lj_epsilon * (sigma_r12 - sigma_r6);
                }
            }
        }
        
        energy
    }
}

/// Amber99SB force field with implicit solvation
#[derive(Debug, Clone)]
pub struct Amber99SBForceField {
    // Bond parameters
    bond_params: HashMap<String, (f64, f64)>, // (kb, r0)
    // Angle parameters
    angle_params: HashMap<String, (f64, f64)>, // (ka, theta0)
    // Dihedral parameters
    dihedral_params: HashMap<String, Vec<(f64, i32, f64)>>, // (kd, n, delta)
    // LJ parameters
    lj_params: HashMap<String, (f64, f64)>, // (sigma, epsilon)
    // Partial charges
    charges: HashMap<String, f64>,
    // GB parameters
    gb_radii: HashMap<String, f64>,
    gb_scaling: HashMap<String, f64>,
}

impl Amber99SBForceField {
    pub fn new() -> Self {
        let mut ff = Self {
            bond_params: HashMap::new(),
            angle_params: HashMap::new(),
            dihedral_params: HashMap::new(),
            lj_params: HashMap::new(),
            charges: HashMap::new(),
            gb_radii: HashMap::new(),
            gb_scaling: HashMap::new(),
        };
        ff.initialize_parameters();
        ff
    }

    fn initialize_parameters(&mut self) {
        // Backbone bonds
        self.bond_params.insert("N-CA".to_string(), (337.0, 1.449));
        self.bond_params.insert("CA-C".to_string(), (317.0, 1.522));
        self.bond_params.insert("C-O".to_string(), (570.0, 1.229));
        self.bond_params.insert("C-N".to_string(), (490.0, 1.335));
        
        // Backbone angles
        self.angle_params.insert("N-CA-C".to_string(), (63.0, 110.1 * PI / 180.0));
        self.angle_params.insert("CA-C-N".to_string(), (70.0, 116.6 * PI / 180.0));
        self.angle_params.insert("C-N-CA".to_string(), (50.0, 121.9 * PI / 180.0));
        
        // Phi/Psi dihedrals
        self.dihedral_params.insert("phi".to_string(), vec![
            (0.2, 1, 0.0),
            (0.2, 2, PI),
            (0.4, 3, 0.0),
        ]);
        self.dihedral_params.insert("psi".to_string(), vec![
            (0.8, 1, 0.0),
            (0.2, 2, PI),
            (0.2, 3, 0.0),
        ]);
        
        // LJ parameters
        self.lj_params.insert("N".to_string(), (3.25, 0.17));
        self.lj_params.insert("CA".to_string(), (3.40, 0.11));
        self.lj_params.insert("C".to_string(), (3.40, 0.086));
        self.lj_params.insert("O".to_string(), (2.96, 0.21));
        
        // Partial charges
        self.charges.insert("N".to_string(), -0.4157);
        self.charges.insert("CA".to_string(), 0.0337);
        self.charges.insert("C".to_string(), 0.5973);
        self.charges.insert("O".to_string(), -0.5679);
        
        // GB radii
        self.gb_radii.insert("N".to_string(), 1.55);
        self.gb_radii.insert("CA".to_string(), 1.70);
        self.gb_radii.insert("C".to_string(), 1.70);
        self.gb_radii.insert("O".to_string(), 1.50);
        
        // GB scaling factors
        self.gb_scaling.insert("N".to_string(), 0.73);
        self.gb_scaling.insert("CA".to_string(), 0.72);
        self.gb_scaling.insert("C".to_string(), 0.72);
        self.gb_scaling.insert("O".to_string(), 0.85);
    }

    pub fn solvation_energy(&self, chain: &PeptideChain) -> f64 {
        let residues = chain.residues();
        let mut energy = 0.0;
        
        // Simplified GB energy calculation
        let dielectric_interior = 1.0;
        let dielectric_exterior = 78.5;
        let prefactor = -332.0 * (1.0 / dielectric_interior - 1.0 / dielectric_exterior);
        
        for (i, res_i) in residues.iter().enumerate() {
            let charge_i = self.charges.get("CA").copied().unwrap_or(0.0);
            let radius_i = self.gb_radii.get("CA").copied().unwrap_or(1.5);
            
            // Self energy
            energy += prefactor * charge_i * charge_i / radius_i;
            
            // Pairwise interactions
            for (j, res_j) in residues.iter().enumerate().skip(i + 1) {
                let charge_j = self.charges.get("CA").copied().unwrap_or(0.0);
                let radius_j = self.gb_radii.get("CA").copied().unwrap_or(1.5);
                
                let pos_i = res_i.position();
                let pos_j = res_j.position();
                let rij = distance(pos_i, pos_j);
                
                let fgb = (rij * rij + radius_i * radius_j * 
                          (-rij * rij / (4.0 * radius_i * radius_j)).exp()).sqrt();
                
                energy += prefactor * charge_i * charge_j / fgb;
            }
        }
        
        energy
    }
}

impl Default for Amber99SBForceField {
    fn default() -> Self {
        Self::new()
    }
}

impl ForceField for Amber99SBForceField {
    fn compute_energy(&self, chain: &PeptideChain) -> f64 {
        self.bond_energy(chain) + 
        self.angle_energy(chain) + 
        self.dihedral_energy(chain) + 
        self.nonbonded_energy(chain) +
        self.solvation_energy(chain)
    }

    fn compute_forces(&self, chain: &PeptideChain) -> Vec<Vec3> {
        // Simplified force calculation - would need numerical derivatives for full implementation
        let residues = chain.residues();
        vec![Vec3::zeros(); residues.len()]
    }

    fn bond_energy(&self, chain: &PeptideChain) -> f64 {
        let residues = chain.residues();
        let mut energy = 0.0;
        
        for i in 0..residues.len().saturating_sub(1) {
            let pos1 = residues[i].position();
            let pos2 = residues[i + 1].position();
            let r = distance(pos1, pos2);
            
            if let Some((kb, r0)) = self.bond_params.get("CA-CA") {
                let dr = r - r0;
                energy += 0.5 * kb * dr * dr;
            }
        }
        
        energy
    }

    fn angle_energy(&self, chain: &PeptideChain) -> f64 {
        let residues = chain.residues();
        let mut energy = 0.0;
        
        for i in 0..residues.len().saturating_sub(2) {
            let pos1 = residues[i].position();
            let pos2 = residues[i + 1].position();
            let pos3 = residues[i + 2].position();
            
            let v1 = [pos1[0] - pos2[0], pos1[1] - pos2[1], pos1[2] - pos2[2]];
            let v2 = [pos3[0] - pos2[0], pos3[1] - pos2[1], pos3[2] - pos2[2]];
            
            let dot = v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2];
            let norm1 = (v1[0] * v1[0] + v1[1] * v1[1] + v1[2] * v1[2]).sqrt();
            let norm2 = (v2[0] * v2[0] + v2[1] * v2[1] + v2[2] * v2[2]).sqrt();
            
            if norm1 > 1e-10 && norm2 > 1e-10 {
                let cos_theta = (dot / (norm1 * norm2)).clamp(-1.0, 1.0);
                let theta = cos_theta.acos();
                
                if let Some((ka, theta0)) = self.angle_params.get("CA-CA-CA") {
                    let dtheta = theta - theta0;
                    energy += 0.5 * ka * dtheta * dtheta;
                }
            }
        }
        
        energy
    }

    fn dihedral_energy(&self, chain: &PeptideChain) -> f64 {
        let residues = chain.residues();
        let mut energy = 0.0;
        
        for i in 1..residues.len().saturating_sub(1) {
            let phi = residues[i].phi;
            let psi = residues[i].psi;
            
            // Phi dihedral
            if let Some(phi_params) = self.dihedral_params.get("phi") {
                for (kd, n, delta) in phi_params {
                    energy += kd * (1.0 + ((*n as f64) * phi + delta).cos());
                }
            }
            
            // Psi dihedral
            if let Some(psi_params) = self.dihedral_params.get("psi") {
                for (kd, n, delta) in psi_params {
                    energy += kd * (1.0 + ((*n as f64) * psi + delta).cos());
                }
            }
        }
        
        energy
    }

    fn nonbonded_energy(&self, chain: &PeptideChain) -> f64 {
        let residues = chain.residues();
        let mut energy = 0.0;
        
        for i in 0..residues.len() {
            for j in (i + 2)..residues.len() { // Skip bonded neighbors
                let pos1 = residues[i].position();
                let pos2 = residues[j].position();
                let r = distance(pos1, pos2);
                
                if r > 1e-10 && r < 12.0 { // Cutoff at 12 Å
                    // Lennard-Jones
                    if let Some((sigma, epsilon)) = self.lj_params.get("CA") {
                        let sigma_r = sigma / r;
                        let sigma_r6 = sigma_r.powi(6);
                        let sigma_r12 = sigma_r6 * sigma_r6;
                        energy += 4.0 * epsilon * (sigma_r12 - sigma_r6);
                    }
                    
                    // Coulomb
                    let q1 = self.charges.get("CA").copied().unwrap_or(0.0);
                    let q2 = self.charges.get("CA").copied().unwrap_or(0.0);
                    energy += 332.0 * q1 * q2 / r; // 332 converts to kcal/mol
                }
            }
        }
        
        energy
    }
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use folding_molecule::{PeptideChain, Residue, ResidueId};

    #[test]
    fn test_coarse_grained_force_field() {
        let ff = CoarseGrainedForceField::new();
        let chain = create_test_chain();
        
        let energy = ff.compute_energy(&chain);
        assert!(energy.is_finite());
        
        let forces = ff.compute_forces(&chain);
        assert_eq!(forces.len(), chain.len());
        
        for force in &forces {
            assert!(force.norm().is_finite());
        }
    }

    #[test]
    fn test_amber99sb_force_field() {
        let ff = Amber99SBForceField::new();
        let chain = create_test_chain();
        
        let energy = ff.compute_energy(&chain);
        assert!(energy.is_finite());
        
        let bond_energy = ff.bond_energy(&chain);
        let angle_energy = ff.angle_energy(&chain);
        let dihedral_energy = ff.dihedral_energy(&chain);
        let nonbonded_energy = ff.nonbonded_energy(&chain);
        let solvation_energy = ff.solvation_energy(&chain);
        
        assert!(bond_energy.is_finite());
        assert!(angle_energy.is_finite());
        assert!(dihedral_energy.is_finite());
        assert!(nonbonded_energy.is_finite());
        assert!(solvation_energy.is_finite());
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
