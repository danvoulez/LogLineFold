use folding_molecule::{BondConstraintSet, PeptideChain, ResidueId};
use folding_time::trajectory::SpanRecord;

/// Validates spans against chemical and informational constraints.
#[derive(Debug, Clone)]
pub struct Ruleset {
    pub max_rotation_degrees: f64,
    pub bond_constraints: BondConstraintSet,
    pub entropy_budget: Option<f64>,
    pub information_budget: Option<f64>,
    pub min_distance_angstrom: Option<f64>,
    pub bond_distance_range: Option<(f64, f64)>,
    pub bond_angle_range: Option<(f64, f64)>,
}

impl Default for Ruleset {
    fn default() -> Self {
        Self {
            max_rotation_degrees: 180.0,
            bond_constraints: BondConstraintSet::default(),
            entropy_budget: None,
            information_budget: None,
            min_distance_angstrom: Some(1.2),
            bond_distance_range: Some((1.2, 1.9)),
            bond_angle_range: Some((1.6, 2.4)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RuleViolation {
    RotationLimitExceeded {
        residue: ResidueId,
        requested: f64,
        limit: f64,
    },
    BondDistanceUnsatisfied {
        residue: ResidueId,
    },
    EntropyBudgetExceeded {
        consumed: f64,
        budget: f64,
    },
    InformationBudgetExceeded {
        consumed: f64,
        budget: f64,
    },
    StructuralClash {
        residue_a: ResidueId,
        residue_b: ResidueId,
        distance: f64,
    },
    MetropolisRejected {
        delta_energy: f64,
    },
    BondLengthOutOfRange {
        residue_left: ResidueId,
        residue_right: ResidueId,
        distance: f64,
        min: f64,
        max: f64,
    },
    BondAngleOutOfRange {
        residue_center: ResidueId,
        angle: f64,
        min: f64,
        max: f64,
    },
}

impl Ruleset {
    pub fn with_rotation_limit(mut self, limit: f64) -> Self {
        self.max_rotation_degrees = limit;
        self
    }

    pub fn with_bonds(mut self, bond_constraints: BondConstraintSet) -> Self {
        self.bond_constraints = bond_constraints;
        self
    }

    pub fn with_entropy_budget(mut self, budget: f64) -> Self {
        self.entropy_budget = Some(budget);
        self
    }

    pub fn with_information_budget(mut self, budget: f64) -> Self {
        self.information_budget = Some(budget);
        self
    }

    pub fn with_min_distance(mut self, distance: f64) -> Self {
        self.min_distance_angstrom = Some(distance);
        self
    }

    pub fn with_bond_distance_range(mut self, range: (f64, f64)) -> Self {
        self.bond_distance_range = Some(range);
        self
    }

    pub fn with_bond_angle_range(mut self, range: (f64, f64)) -> Self {
        self.bond_angle_range = Some(range);
        self
    }

    pub fn validate_rotation(
        &self,
        residue: ResidueId,
        angle: f64,
        _chain: &PeptideChain,
    ) -> Result<(), RuleViolation> {
        if angle.abs() > self.max_rotation_degrees {
            return Err(RuleViolation::RotationLimitExceeded {
                residue,
                requested: angle,
                limit: self.max_rotation_degrees,
            });
        }
        // Bond constraint evaluation is stubbed for now.
        let _ = &self.bond_constraints;
        Ok(())
    }

    pub fn check_budgets(
        &self,
        span: &SpanRecord,
        consumed_entropy: f64,
        consumed_information: f64,
    ) -> Result<(), RuleViolation> {
        if let Some(budget) = self.entropy_budget {
            let projected = consumed_entropy + span.delta_entropy;
            if projected > budget {
                return Err(RuleViolation::EntropyBudgetExceeded {
                    consumed: projected,
                    budget,
                });
            }
        }
        if let Some(budget) = self.information_budget {
            let projected = consumed_information + span.delta_information;
            if projected > budget {
                return Err(RuleViolation::InformationBudgetExceeded {
                    consumed: projected,
                    budget,
                });
            }
        }
        Ok(())
    }

    pub fn check_structure(&self, chain: &PeptideChain) -> Result<(), RuleViolation> {
        if let Some(min_distance) = self.min_distance_angstrom {
            let residues = chain.residues();
            for (i, left) in residues.iter().enumerate() {
                for right in residues.iter().skip(i + 1) {
                    let distance = distance(left.position(), right.position());
                    if distance < min_distance {
                        return Err(RuleViolation::StructuralClash {
                            residue_a: left.id,
                            residue_b: right.id,
                            distance,
                        });
                    }
                }
            }
        }
        if let Some((min, max)) = self.bond_distance_range {
            let residues = chain.residues();
            for window in residues.windows(2) {
                if let [left, right] = window {
                    let distance = distance(left.position(), right.position());
                    if distance < min || distance > max {
                        return Err(RuleViolation::BondLengthOutOfRange {
                            residue_left: left.id,
                            residue_right: right.id,
                            distance,
                            min,
                            max,
                        });
                    }
                }
            }
        }
        if let Some((min, max)) = self.bond_angle_range {
            let residues = chain.residues();
            for window in residues.windows(3) {
                if let [left, center, right] = window {
                    let angle = bond_angle(left.position(), center.position(), right.position());
                    if angle < min || angle > max {
                        return Err(RuleViolation::BondAngleOutOfRange {
                            residue_center: center.id,
                            angle,
                            min,
                            max,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use folding_molecule::{AminoAcid, PeptideChain, Residue, ResidueId};

    #[test]
    fn detects_structural_clash_when_residues_overlap() {
        let residues = vec![
            Residue::new(ResidueId(0), AminoAcid::Alanine).with_position([0.0, 0.0, 0.0]),
            Residue::new(ResidueId(1), AminoAcid::Glycine).with_position([0.1, 0.0, 0.0]),
        ];
        let chain = PeptideChain::new(residues);
        let ruleset = Ruleset::default().with_min_distance(0.5);
        let result = ruleset.check_structure(&chain);
        assert!(matches!(result, Err(RuleViolation::StructuralClash { .. })));
    }

    #[test]
    fn detects_bond_length_out_of_range() {
        let residues = vec![
            Residue::new(ResidueId(0), AminoAcid::Alanine).with_position([0.0, 0.0, 0.0]),
            Residue::new(ResidueId(1), AminoAcid::Glycine).with_position([2.5, 0.0, 0.0]),
        ];
        let chain = PeptideChain::new(residues);
        let ruleset = Ruleset::default().with_bond_distance_range((1.0, 2.0));
        let result = ruleset.check_structure(&chain);
        assert!(matches!(
            result,
            Err(RuleViolation::BondLengthOutOfRange { .. })
        ));
    }
}

fn distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    ((left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2) + (left[2] - right[2]).powi(2))
        .sqrt()
}

fn bond_angle(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let v1 = subtract(a, b);
    let v2 = subtract(c, b);
    let dot = dot(v1, v2);
    let norm = (norm(v1) * norm(v2)).max(1e-9);
    (dot / norm).clamp(-1.0, 1.0).acos()
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0].powi(2) + v[1].powi(2) + v[2].powi(2)).sqrt()
}
