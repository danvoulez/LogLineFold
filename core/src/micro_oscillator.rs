use std::f64::consts::PI;
use std::time::Duration;

/// Generates micro/millisecond oscillations used to perturb rotation angles.
#[derive(Debug, Clone)]
pub struct MicroOscillator {
    frequency_hz: f64,
    amplitude: f64,
}

impl MicroOscillator {
    pub fn new(frequency_hz: f64, amplitude: f64) -> Self {
        Self {
            frequency_hz,
            amplitude,
        }
    }

    pub fn sample(&self, elapsed: Duration) -> f64 {
        let t = elapsed.as_secs_f64();
        (2.0 * PI * self.frequency_hz * t).sin() * self.amplitude
    }
}
