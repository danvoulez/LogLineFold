use std::f64::consts::PI;


/// Identifier for a residue within a peptide chain.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ResidueId(pub usize);

/// Simplified amino acid catalogue used for tests and defaults.
#[derive(Clone, Copy, Debug)]
pub enum AminoAcid {
    Alanine,
    Glycine,
    Serine,
    Valine,
}

impl AminoAcid {
    fn code(self) -> &'static str {
        match self {
            AminoAcid::Alanine => "ALA",
            AminoAcid::Glycine => "GLY",
            AminoAcid::Serine => "SER",
            AminoAcid::Valine => "VAL",
        }
    }
}

impl From<AminoAcid> for String {
    fn from(value: AminoAcid) -> Self {
        value.code().to_string()
    }
}

/// Representation of a single amino-acid residue with coarse coordinates.
#[derive(Clone, Debug)]
pub struct Residue {
    pub id: ResidueId,
    pub name: String,
    pub phi: f64,
    pub psi: f64,
    position: [f64; 3],
}

impl Residue {
    pub fn new(id: ResidueId, name: impl Into<String>, position: [f64; 3]) -> Self {
        Self {
            id,
            name: name.into(),
            phi: 0.0,
            psi: 0.0,
            position,
        }
    }

    pub fn with_position(mut self, position: [f64; 3]) -> Self {
        self.position = position;
        self
    }

    pub fn position(&self) -> [f64; 3] {
        self.position
    }

    pub fn set_position(&mut self, position: [f64; 3]) {
        self.position = position;
    }
}

/// Minimal bond constraint representation.
#[derive(Clone, Debug)]
pub struct BondConstraintSet {
    pub preferred_distance: f64,
}

impl Default for BondConstraintSet {
    fn default() -> Self {
        Self {
            preferred_distance: 3.8,
        }
    }
}

/// Simplified peptide chain with evenly spaced residues.
#[derive(Clone, Debug, Default)]
pub struct PeptideChain {
    residues: Vec<Residue>,
}

impl PeptideChain {
    pub fn new(residues: Vec<Residue>) -> Self {
        Self { residues }
    }

    pub fn from_sequence(sequence: &str) -> Self {
        let residues = sequence
            .chars()
            .enumerate()
            .map(|(idx, symbol)| {
                let name = amino_acid_three_letter(symbol);
                let angle = idx as f64 * (PI / 8.0);
                let radius = 5.0;
                let position = [radius * angle.cos(), radius * angle.sin(), idx as f64 * 1.5];
                Residue::new(ResidueId(idx), name, position)
            })
            .collect();
        Self { residues }
    }

    pub fn residues(&self) -> &[Residue] {
        &self.residues
    }

    pub fn residues_mut(&mut self) -> &mut [Residue] {
        &mut self.residues
    }

    pub fn residue(&self, id: ResidueId) -> Option<&Residue> {
        self.residues.get(id.0)
    }

    pub fn residue_mut(&mut self, id: ResidueId) -> Option<&mut Residue> {
        self.residues.get_mut(id.0)
    }

    pub fn len(&self) -> usize {
        self.residues.len()
    }

    pub fn is_empty(&self) -> bool {
        self.residues.is_empty()
    }
}

/// Aggregated energy components.
#[derive(Clone, Debug, Default)]
pub struct EnergySummary {
    pub potential: f64,
}

impl EnergySummary {
    pub fn total(&self) -> f64 {
        self.potential
    }
}

/// Lightweight energy model that penalises bond stretching and steric clashes.
#[derive(Clone, Debug)]
pub struct EnergyModel {
    bond_strength: f64,
    steric_repulsion: f64,
}

impl Default for EnergyModel {
    fn default() -> Self {
        Self {
            bond_strength: 1.0,
            steric_repulsion: 0.1,
        }
    }
}

impl EnergyModel {
    pub fn total_energy(&self, chain: &PeptideChain) -> f64 {
        self.energy_summary(chain).total()
    }

    pub fn energy_summary(&self, chain: &PeptideChain) -> EnergySummary {
        let mut potential = 0.0;
        for window in chain.residues().windows(2) {
            if let [left, right] = window {
                let dist = distance(left.position(), right.position());
                let stretch = dist - 3.8;
                potential += 0.5 * self.bond_strength * stretch * stretch;
            }
        }
        for (i, residue) in chain.residues().iter().enumerate() {
            for other in chain.residues().iter().skip(i + 1) {
                let dist = distance(residue.position(), other.position());
                if dist > 0.0 {
                    potential += self.steric_repulsion / dist.powi(12);
                }
            }
        }
        EnergySummary { potential }
    }
}

fn amino_acid_three_letter(symbol: char) -> String {
    match symbol.to_ascii_uppercase() {
        'A' => "ALA",
        'C' => "CYS",
        'D' => "ASP",
        'E' => "GLU",
        'F' => "PHE",
        'G' => "GLY",
        'H' => "HIS",
        'I' => "ILE",
        'K' => "LYS",
        'L' => "LEU",
        'M' => "MET",
        'N' => "ASN",
        'P' => "PRO",
        'Q' => "GLN",
        'R' => "ARG",
        'S' => "SER",
        'T' => "THR",
        'V' => "VAL",
        'W' => "TRP",
        'Y' => "TYR",
        _ => "UNK",
    }
    .to_string()
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_from_sequence_has_expected_length() {
        let chain = PeptideChain::from_sequence("ACDE");
        assert_eq!(chain.len(), 4);
        assert_eq!(chain.residue(ResidueId(0)).unwrap().name, "ALA");
        assert_eq!(chain.residue(ResidueId(2)).unwrap().name, "ASP");
    }

    #[test]
    fn energy_model_reports_reasonable_total() {
        let chain = PeptideChain::from_sequence("AAAA");
        let model = EnergyModel::default();
        let energy = model.total_energy(&chain);
        assert!(energy.is_finite());
        assert!(energy >= 0.0);
    }
}
