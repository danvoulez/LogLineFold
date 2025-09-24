use folding_molecule::{PeptideChain, ResidueId};
use folding_time::trajectory::{SpanRecord, Trajectory};

use crate::folding_ruleset::{RuleViolation, Ruleset};

/// Events emitted by the validator during enforcement.
#[derive(Debug, Clone)]
pub enum ValidationEvent {
    Accepted,
    Rejected(RuleViolation),
}

/// Validates spans and propagates rule violations.
pub struct Validator {
    ruleset: Ruleset,
}

impl Validator {
    pub fn new(ruleset: Ruleset) -> Self {
        Self { ruleset }
    }

    pub fn validate_rotation(
        &self,
        residue: ResidueId,
        angle: f64,
        chain: &PeptideChain,
    ) -> Result<(), RuleViolation> {
        self.ruleset.validate_rotation(residue, angle, chain)
    }

    pub fn validate_span(&self, span: &SpanRecord, trajectory: &Trajectory) -> ValidationEvent {
        let entropy = trajectory.total_entropy();
        let information = trajectory.total_information();
        match self.ruleset.check_budgets(span, entropy, information) {
            Ok(()) => ValidationEvent::Accepted,
            Err(err) => ValidationEvent::Rejected(err),
        }
    }

    pub fn validate_structure(&self, chain: &PeptideChain) -> Result<(), RuleViolation> {
        self.ruleset.check_structure(chain)
    }
}
