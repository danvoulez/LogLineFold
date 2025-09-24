use std::time::Duration;

pub mod trajectory {
    use super::Duration;

    /// SpanRecord captures entropy/information deltas for a single rotation.
    #[derive(Clone, Debug)]
    pub struct SpanRecord {
        pub id: String,
        pub delta_entropy: f64,
        pub delta_information: f64,
        pub duration: Duration,
        pub delta_theta: f64,
        pub delta_energy: f64,
        pub gibbs_energy: f64,
    }

    impl SpanRecord {
        pub fn new(
            id: impl Into<String>,
            delta_entropy: f64,
            delta_information: f64,
            duration: Duration,
        ) -> Self {
            Self {
                id: id.into(),
                delta_entropy,
                delta_information,
                duration,
                delta_theta: 0.0,
                delta_energy: 0.0,
                gibbs_energy: 0.0,
            }
        }
    }

    /// Ordered sequence of executed spans.
    #[derive(Clone, Debug, Default)]
    pub struct Trajectory {
        spans: Vec<SpanRecord>,
    }

    impl Trajectory {
        pub fn new() -> Self {
            Self { spans: Vec::new() }
        }

        pub fn push(&mut self, span: SpanRecord) {
            self.spans.push(span);
        }

        pub fn pop_last(&mut self) -> Option<SpanRecord> {
            self.spans.pop()
        }

        pub fn total_entropy(&self) -> f64 {
            self.spans.iter().map(|span| span.delta_entropy).sum()
        }

        pub fn total_information(&self) -> f64 {
            self.spans.iter().map(|span| span.delta_information).sum()
        }

        pub fn iter(&self) -> impl Iterator<Item = &SpanRecord> {
            self.spans.iter()
        }

        pub fn is_empty(&self) -> bool {
            self.spans.is_empty()
        }

        pub fn len(&self) -> usize {
            self.spans.len()
        }
    }
}

/// Wall-clock pacing for rotations.
#[derive(Clone, Debug)]
pub struct RotationClock {
    tick_duration: Duration,
    counter: u64,
}

impl RotationClock {
    pub fn new(step_ms: u64) -> Self {
        Self {
            tick_duration: Duration::from_millis(step_ms.max(1)),
            counter: 0,
        }
    }

    pub fn tick_duration(&self) -> Duration {
        self.tick_duration
    }

    pub fn tick(&mut self) -> Duration {
        self.counter = self.counter.wrapping_add(1);
        self.tick_duration
    }
}

#[cfg(test)]
mod tests {
    use super::trajectory::SpanRecord;
    use super::*;

    #[test]
    fn trajectory_accumulates_entropy() {
        let mut traj = trajectory::Trajectory::new();
        traj.push(SpanRecord::new("a", 0.5, 0.2, Duration::from_millis(1)));
        traj.push(SpanRecord::new("b", 0.2, 0.1, Duration::from_millis(1)));
        assert!((traj.total_entropy() - 0.7).abs() < 1e-9);
    }

    #[test]
    fn rotation_clock_advances_ticks() {
        let mut clock = RotationClock::new(2);
        assert_eq!(clock.tick_duration(), Duration::from_millis(2));
        assert_eq!(clock.tick(), Duration::from_millis(2));
    }
}
