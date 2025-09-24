use std::time::Duration;

use folding_molecule::{EnergyModel, PeptideChain, ResidueId};
use folding_time::{RotationClock, Trajectory};
use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::folding_parser::{ContractInstruction, FoldingContract, PhysicsLevel, PhysicsSpanMode};
use crate::folding_ruleset::{RuleViolation, Ruleset};
use crate::micro_oscillator::MicroOscillator;
use crate::physics_bridge::{self, PhysicsRequest, PhysicsSpanMetrics};
use crate::protein_state::{EnergyState, ProteinSnapshot, ProteinState};
use crate::rotation_solver::{RotationCommand, RotationOutcome, RotationSolver};
use crate::validation::{ValidationEvent, Validator};

#[derive(Clone, Debug)]
pub enum TemperatureSchedule {
    Constant,
    Linear { start: f64, end: f64, steps: usize },
}

impl TemperatureSchedule {
    pub fn temperature_for_step(&self, step: usize, initial: f64) -> f64 {
        match self {
            TemperatureSchedule::Constant => initial,
            TemperatureSchedule::Linear { start, end, steps } => {
                if *steps == 0 {
                    *end
                } else {
                    let ratio = (step.min(*steps) as f64) / (*steps as f64);
                    start + (end - start) * ratio
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MetropolisStats {
    pub accepted: usize,
    pub rejected: usize,
}

impl MetropolisStats {
    pub fn record_accept(&mut self) {
        self.accepted += 1;
    }

    pub fn record_reject(&mut self) {
        self.rejected += 1;
    }

    pub fn total(&self) -> usize {
        self.accepted + self.rejected
    }

    pub fn acceptance_rate(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.accepted as f64 / total as f64
        }
    }
}

/// Primary engine orchestrating the folding session.
pub struct FoldingEngine {
    state: ProteinState,
    solver: RotationSolver,
    validator: Validator,
    ghost_mode: bool,
    pending_alias: Option<String>,
    checkpoints: Vec<ProteinSnapshot>,
    ghost_trajectory: Trajectory,
    temperature: f64,
    boltzmann_constant: f64,
    rng: StdRng,
    temperature_schedule: Option<TemperatureSchedule>,
    initial_temperature: f64,
    step_index: usize,
    metropolis_stats: MetropolisStats,
    domains: Vec<DomainDefinition>,
    chaperone_requirements: Vec<ChaperoneRequirement>,
    modifications: Vec<PostTranslationalModification>,
    physics_level: PhysicsLevel,
    span_physics_mode: PhysicsSpanMode,
    physics_spans: Vec<String>,
    physics_span_metrics: Vec<PhysicsSpanRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_schedule_returns_initial() {
        let schedule = TemperatureSchedule::Constant;
        assert_eq!(schedule.temperature_for_step(10, 300.0), 300.0);
    }

    #[test]
    fn linear_schedule_interpolates() {
        let schedule = TemperatureSchedule::Linear {
            start: 400.0,
            end: 300.0,
            steps: 10,
        };
        assert!((schedule.temperature_for_step(0, 400.0) - 400.0).abs() < 1e-6);
        assert!((schedule.temperature_for_step(5, 400.0) - 350.0).abs() < 1e-6);
        assert!((schedule.temperature_for_step(10, 400.0) - 300.0).abs() < 1e-6);
        assert!((schedule.temperature_for_step(20, 400.0) - 300.0).abs() < 1e-6);
    }
}

pub struct FoldingEngineBuilder {
    chain: Option<PeptideChain>,
    energy_model: Option<EnergyModel>,
    oscillator: Option<MicroOscillator>,
    clock: Option<RotationClock>,
    ruleset: Option<Ruleset>,
    temperature: Option<f64>,
    rng_seed: Option<u64>,
    temperature_schedule: Option<TemperatureSchedule>,
    physics_level: Option<PhysicsLevel>,
}

pub struct ExecutionReport {
    pub applied_rotations: Vec<RotationOutcome>,
    pub ghost_rotations: Vec<RotationOutcome>,
    pub rejections: Vec<RuleViolation>,
    pub final_energy: EnergyState,
    pub trajectory: Trajectory,
    pub metropolis_stats: MetropolisStats,
    pub domains: Vec<DomainDefinition>,
    pub chaperone_requirements: Vec<ChaperoneRequirement>,
    pub modifications: Vec<PostTranslationalModification>,
    pub physics_level: PhysicsLevel,
    pub physics_spans: Vec<String>,
    pub physics_span_metrics: Vec<PhysicsSpanRecord>,
}

#[derive(Clone, Debug)]
pub struct DomainDefinition {
    pub name: Option<String>,
    pub start: ResidueId,
    pub end: ResidueId,
}

#[derive(Clone, Debug)]
pub struct ChaperoneRequirement {
    pub chaperone: String,
    pub span: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PostTranslationalModification {
    pub modification: String,
    pub residue: ResidueId,
}

#[derive(Clone, Debug)]
pub struct PhysicsSpanRecord {
    pub span_id: String,
    pub metrics: PhysicsSpanMetrics,
}

impl FoldingEngineBuilder {
    pub fn new() -> Self {
        Self {
            chain: None,
            energy_model: None,
            oscillator: None,
            clock: None,
            ruleset: None,
            temperature: None,
            rng_seed: None,
            temperature_schedule: None,
            physics_level: None,
        }
    }

    pub fn with_chain(mut self, chain: PeptideChain) -> Self {
        self.chain = Some(chain);
        self
    }

    pub fn with_energy_model(mut self, energy_model: EnergyModel) -> Self {
        self.energy_model = Some(energy_model);
        self
    }

    pub fn with_oscillator(mut self, oscillator: MicroOscillator) -> Self {
        self.oscillator = Some(oscillator);
        self
    }

    pub fn with_clock(mut self, clock: RotationClock) -> Self {
        self.clock = Some(clock);
        self
    }

    pub fn with_ruleset(mut self, ruleset: Ruleset) -> Self {
        self.ruleset = Some(ruleset);
        self
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_rng_seed(mut self, seed: u64) -> Self {
        self.rng_seed = Some(seed);
        self
    }

    pub fn with_temperature_schedule(mut self, schedule: TemperatureSchedule) -> Self {
        self.temperature_schedule = Some(schedule);
        self
    }

    pub fn with_physics_level(mut self, level: PhysicsLevel) -> Self {
        self.physics_level = Some(level);
        self
    }

    pub fn build(self) -> FoldingEngine {
        let chain = self.chain.expect("chain not provided");
        let energy_model = self.energy_model.unwrap_or_default();
        let oscillator = self
            .oscillator
            .unwrap_or_else(|| MicroOscillator::new(10.0, 1.0));
        let clock = self.clock.unwrap_or_else(|| RotationClock::new(1));
        let ruleset = self.ruleset.unwrap_or_default();
        let mut temperature = self.temperature.unwrap_or(310.0);
        let temperature_schedule = self.temperature_schedule.clone();
        if let Some(TemperatureSchedule::Linear { start, .. }) = &temperature_schedule {
            temperature = *start;
        }
        let rng = match self.rng_seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_entropy(),
        };
        let state = ProteinState::new(chain, energy_model);
        let solver = RotationSolver::new(oscillator, clock);
        let validator = Validator::new(ruleset);
        let physics_level = self.physics_level.unwrap_or(PhysicsLevel::Toy);
        FoldingEngine {
            state,
            solver,
            validator,
            ghost_mode: false,
            pending_alias: None,
            checkpoints: Vec::new(),
            ghost_trajectory: Trajectory::new(),
            temperature,
            boltzmann_constant: 0.0019872041, // kcal·mol⁻¹·K⁻¹
            rng,
            temperature_schedule,
            initial_temperature: temperature,
            step_index: 0,
            metropolis_stats: MetropolisStats::default(),
            domains: Vec::new(),
            chaperone_requirements: Vec::new(),
            modifications: Vec::new(),
            physics_level,
            span_physics_mode: PhysicsSpanMode::Toy,
            physics_spans: Vec::new(),
            physics_span_metrics: Vec::new(),
        }
    }
}

impl FoldingEngine {
    pub fn execute_contract(&mut self, contract: &FoldingContract) -> ExecutionReport {
        let mut applied_rotations = Vec::new();
        let mut ghost_rotations = Vec::new();
        let mut rejections = Vec::new();
        self.step_index = 0;
        self.metropolis_stats = MetropolisStats::default();
        self.domains.clear();
        self.chaperone_requirements.clear();
        self.modifications.clear();
        self.physics_spans.clear();
        self.physics_span_metrics.clear();
        for instruction in &contract.instructions {
            match instruction {
                ContractInstruction::Rotate {
                    residue,
                    angle_degrees,
                    duration_ms,
                } => match self.execute_rotation(*residue, *angle_degrees, *duration_ms) {
                    Ok(outcome) => {
                        if outcome.ghost {
                            ghost_rotations.push(outcome);
                        } else {
                            applied_rotations.push(outcome);
                        }
                    }
                    Err(err) => rejections.push(err),
                },
                ContractInstruction::ClashCheck => {
                    if let Err(err) = self.validator.validate_structure(&self.state.chain) {
                        rejections.push(err);
                    }
                }
                ContractInstruction::Commit => self.commit(),
                ContractInstruction::Rollback => self.rollback(),
                ContractInstruction::GhostMode(enabled) => self.set_ghost_mode(*enabled),
                ContractInstruction::SpanAlias(alias) => {
                    self.pending_alias = Some(alias.clone());
                }
                ContractInstruction::DefineDomain { name, start, end } => {
                    self.domains.push(DomainDefinition {
                        name: name.clone(),
                        start: *start,
                        end: *end,
                    });
                }
                ContractInstruction::RequireChaperone { chaperone, span } => {
                    self.chaperone_requirements.push(ChaperoneRequirement {
                        chaperone: chaperone.clone(),
                        span: span.clone(),
                    });
                }
                ContractInstruction::AddModification {
                    modification,
                    residue,
                } => {
                    self.modifications.push(PostTranslationalModification {
                        modification: modification.clone(),
                        residue: *residue,
                    });
                }
                ContractInstruction::SetPhysicsLevel(level) => {
                    self.physics_level = *level;
                }
                ContractInstruction::SetSpanPhysics(mode) => {
                    self.span_physics_mode = *mode;
                }
            }
        }
        let final_energy = self.state.energy_state();
        let trajectory = self.state.trajectory().clone();
        ExecutionReport {
            applied_rotations,
            ghost_rotations,
            rejections,
            final_energy,
            trajectory,
            metropolis_stats: self.metropolis_stats.clone(),
            domains: self.domains.clone(),
            chaperone_requirements: self.chaperone_requirements.clone(),
            modifications: self.modifications.clone(),
            physics_level: self.physics_level,
            physics_spans: self.physics_spans.clone(),
            physics_span_metrics: self.physics_span_metrics.clone(),
        }
    }

    fn execute_rotation(
        &mut self,
        residue: ResidueId,
        angle_degrees: f64,
        duration_ms: u64,
    ) -> Result<RotationOutcome, RuleViolation> {
        self.apply_temperature_schedule();
        self.validator
            .validate_rotation(residue, angle_degrees, &self.state.chain)?;
        let alias = self.pending_alias.take();
        let baseline_energy = self.state.energy_model.total_energy(&self.state.chain);
        let command = RotationCommand {
            residue,
            angle_degrees,
            duration: Duration::from_millis(duration_ms.max(1)),
            label: alias.clone(),
        };
        let mut physics_applied = false;
        let mut outcome = if self.span_physics_mode == PhysicsSpanMode::Physics {
            if let Some(physics_outcome) = physics_bridge::run_physics_step(PhysicsRequest {
                chain: &self.state.chain,
                command: command.clone(),
                level: self.physics_level,
                temperature: self.temperature,
            }) {
                physics_applied = true;
                physics_outcome
            } else {
                self.solver.solve(command.clone())
            }
        } else {
            self.solver.solve(command.clone())
        };
        let pending_metrics = if physics_applied {
            outcome.physics_metrics.clone()
        } else {
            None
        };
        if self.ghost_mode {
            outcome.ghost = true;
            self.ghost_trajectory.push(outcome.span_record.clone());
            self.increment_step();
            return Ok(outcome);
        }
        let span_validation = {
            let trajectory = self.state.trajectory();
            self.validator
                .validate_span(&outcome.span_record, trajectory)
        };
        if let ValidationEvent::Rejected(err) = span_validation {
            self.pending_alias = alias;
            self.increment_step();
            return Err(err);
        }

        let snapshot = self.state.snapshot();
        self.state.apply_rotation(residue, outcome.applied_angle);
        if let Err(err) = self.validator.validate_structure(&self.state.chain) {
            self.state.restore(snapshot);
            self.pending_alias = alias;
            self.increment_step();
            return Err(err);
        }

        let new_energy = self.state.energy_model.total_energy(&self.state.chain);
        let delta_energy = new_energy - baseline_energy;
        let projected_entropy =
            self.state.trajectory().total_entropy() + outcome.span_record.delta_entropy;
        let projected_gibbs = new_energy - self.temperature * projected_entropy;
        outcome.span_record.delta_energy = delta_energy;
        outcome.span_record.gibbs_energy = projected_gibbs;
        if delta_energy > 0.0 {
            let beta = 1.0 / (self.boltzmann_constant * self.temperature.max(1.0));
            let exponent = (-delta_energy * beta).clamp(-700.0, 50.0);
            let acceptance = exponent.exp().min(1.0);
            let roll: f64 = self.rng.gen_range(0.0..1.0);
            if roll >= acceptance {
                self.state.restore(snapshot);
                self.pending_alias = alias;
                self.metropolis_stats.record_reject();
                self.increment_step();
                return Err(RuleViolation::MetropolisRejected { delta_energy });
            }
        }

        self.metropolis_stats.record_accept();

        let trajectory = self.state.trajectory_mut();
        trajectory.push(outcome.span_record.clone());
        if physics_applied {
            let span_id = outcome.span_record.id.as_str().to_string();
            self.physics_spans.push(span_id.clone());
            if let Some(metrics) = pending_metrics {
                self.physics_span_metrics
                    .push(PhysicsSpanRecord { span_id, metrics });
            }
        }
        self.increment_step();
        Ok(outcome)
    }

    fn commit(&mut self) {
        self.checkpoints.push(self.state.snapshot());
    }

    fn rollback(&mut self) {
        if let Some(snapshot) = self.checkpoints.pop() {
            self.state.restore(snapshot);
        } else {
            let _ = self.state.trajectory_mut().pop_last();
        }
    }

    fn set_ghost_mode(&mut self, enabled: bool) {
        self.ghost_mode = enabled;
        if !enabled {
            self.ghost_trajectory = Trajectory::new();
        }
    }

    pub fn trajectory(&self) -> &Trajectory {
        self.state.trajectory()
    }

    fn apply_temperature_schedule(&mut self) {
        if let Some(schedule) = &self.temperature_schedule {
            let new_temp = schedule.temperature_for_step(self.step_index, self.initial_temperature);
            self.temperature = new_temp;
        }
    }

    fn increment_step(&mut self) {
        self.step_index = self.step_index.saturating_add(1);
    }
}
