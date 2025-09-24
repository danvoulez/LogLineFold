use folding_core::ExecutionReport;
use folding_time::trajectory::Trajectory;

/// Aggregate metrics derived from a folding execution report.
#[derive(Debug, Default)]
pub struct FoldingMetrics {
    pub total_entropy: f64,
    pub ghost_entropy: f64,
    pub total_information: f64,
    pub ghost_information: f64,
    pub applied_spans: usize,
    pub ghost_spans: usize,
}

impl FoldingMetrics {
    pub fn from_report(report: &ExecutionReport) -> Self {
        let total_entropy = sum_entropy(&report.applied_rotations);
        let ghost_entropy = sum_entropy(&report.ghost_rotations);
        let total_information = sum_information(&report.applied_rotations);
        let ghost_information = sum_information(&report.ghost_rotations);
        Self {
            total_entropy,
            ghost_entropy,
            total_information,
            ghost_information,
            applied_spans: report.applied_rotations.len(),
            ghost_spans: report.ghost_rotations.len(),
        }
    }
}

fn sum_entropy(spans: &[folding_core::RotationOutcome]) -> f64 {
    spans
        .iter()
        .map(|outcome| outcome.span_record.delta_entropy)
        .sum()
}

fn sum_information(spans: &[folding_core::RotationOutcome]) -> f64 {
    spans
        .iter()
        .map(|outcome| outcome.span_record.delta_information)
        .sum()
}

/// Serialises a trajectory into a compact JSON string for UI consumption.
pub struct TrajectoryVisualizer;

impl TrajectoryVisualizer {
    pub fn to_json(trajectory: &Trajectory) -> String {
        let mut output = String::from("[");
        for (index, span) in trajectory.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&format!(
                "{{\"index\":{},\"id\":\"{}\",\"delta_entropy\":{},\"delta_information\":{},\"delta_theta\":{},\"delta_energy\":{},\"gibbs_energy\":{},\"duration_ms\":{}}}",
                index,
                escape_json(&span.id),
                span.delta_entropy,
                span.delta_information,
                span.delta_theta,
                span.delta_energy,
                span.gibbs_energy,
                span.duration.as_millis()
            ));
        }
        output.push(']');
        output
    }
}

fn escape_json(input: &str) -> String {
    input.replace('\"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use folding_core::rotation_solver::RotationOutcome;
    use folding_time::trajectory::SpanRecord;
    use std::time::Duration;

    #[test]
    fn metrics_compute_totals() {
        let mut report = ExecutionReport {
            applied_rotations: Vec::new(),
            ghost_rotations: Vec::new(),
            rejections: Vec::new(),
            final_energy: folding_core::EnergyState::default(),
            trajectory: Trajectory::new(),
            metropolis_stats: folding_core::MetropolisStats::default(),
            domains: Vec::new(),
            chaperone_requirements: Vec::new(),
            modifications: Vec::new(),
            physics_level: folding_core::PhysicsLevel::Toy,
            physics_spans: Vec::new(),
            physics_span_metrics: Vec::new(),
        };
        report.applied_rotations.push(RotationOutcome {
            applied_angle: 1.0,
            span_record: SpanRecord {
                id: "a".into(),
                delta_entropy: 0.5,
                delta_information: 0.2,
                duration: Duration::from_millis(1),
                delta_theta: 1.0,
                delta_energy: -0.1,
                gibbs_energy: -0.1,
            },
            ghost: false,
            physics_metrics: None,
        });
        let metrics = FoldingMetrics::from_report(&report);
        assert!((metrics.total_entropy - 0.5).abs() < 1e-9);
        assert_eq!(metrics.applied_spans, 1);
    }

    #[test]
    fn trajectory_serialises_to_json() {
        let mut trajectory = Trajectory::new();
        trajectory.push(SpanRecord {
            id: "a".into(),
            delta_entropy: 0.5,
            delta_information: 0.1,
            duration: Duration::from_millis(5),
            delta_theta: 1.2,
            delta_energy: -0.1,
            gibbs_energy: -0.3,
        });
        let json = TrajectoryVisualizer::to_json(&trajectory);
        assert!(json.contains("\"id\":\"a\""));
        assert!(json.contains("\"duration_ms\":5"));
    }
}
