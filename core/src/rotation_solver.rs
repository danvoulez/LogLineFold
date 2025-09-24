use std::time::Duration;

use folding_molecule::ResidueId;
use folding_time::RotationClock;
use folding_time::trajectory::SpanRecord;

use crate::micro_oscillator::MicroOscillator;
use crate::physics_bridge::PhysicsSpanMetrics;

/// Command describing the desired rotation.
#[derive(Debug, Clone)]
pub struct RotationCommand {
    pub residue: ResidueId,
    pub angle_degrees: f64,
    pub duration: Duration,
    pub label: Option<String>,
}

/// Result from executing a rotation step.
#[derive(Debug, Clone)]
pub struct RotationOutcome {
    pub applied_angle: f64,
    pub span_record: SpanRecord,
    pub ghost: bool,
    pub physics_metrics: Option<PhysicsSpanMetrics>,
}

/// Calculates the final rotation after applying oscillations and clock pacing.
pub struct RotationSolver {
    oscillator: MicroOscillator,
    clock: RotationClock,
}

impl RotationSolver {
    pub fn new(oscillator: MicroOscillator, clock: RotationClock) -> Self {
        Self { oscillator, clock }
    }

    pub fn solve(&self, command: RotationCommand) -> RotationOutcome {
        let oscillation = self.oscillator.sample(command.duration);
        let applied_angle = command.angle_degrees + oscillation;
        let id = command
            .label
            .unwrap_or_else(|| format!("residue-{}", command.residue.0));
        let span_duration = if command.duration.is_zero() {
            self.clock.tick_duration()
        } else {
            command.duration
        };
        let mut span = SpanRecord::new(
            id,
            applied_angle.abs() * 0.01,
            applied_angle.abs() * 0.005,
            span_duration,
        );
        span.delta_theta = applied_angle;
        RotationOutcome {
            applied_angle,
            span_record: span,
            ghost: false,
            physics_metrics: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_rotation_and_records_span() {
        let solver = RotationSolver::new(MicroOscillator::new(0.0, 0.0), RotationClock::new(1));
        let command = RotationCommand {
            residue: ResidueId(1),
            angle_degrees: 30.0,
            duration: Duration::from_millis(4),
            label: Some("alias-span".into()),
        };

        let outcome = solver.solve(command);

        assert!((outcome.applied_angle - 30.0).abs() < f64::EPSILON);
        assert_eq!(outcome.span_record.id.as_str(), "alias-span");
        assert_eq!(outcome.span_record.duration, Duration::from_millis(4));
        assert!(!outcome.ghost);
    }
}
