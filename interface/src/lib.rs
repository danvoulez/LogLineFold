use folding_core::{
    ContractInstruction, ExecutionReport, FoldingContract, FoldingEngineBuilder, MetropolisStats,
    TemperatureSchedule,
};
use folding_molecule::{EnergyModel, PeptideChain};
use folding_sim::FoldingMetrics;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for a folding shell invocation.
#[derive(Clone, Debug)]
pub struct ShellConfig {
    pub temperature: f64,
    pub time_step_ms: u64,
    pub rng_seed: Option<u64>,
    pub log_path: Option<PathBuf>,
    pub environment: String,
    pub diamond_threshold: Option<f64>,
    pub diamond_path: Option<PathBuf>,
    pub temp_schedule: Option<TempScheduleConfig>,
}

/// Linear annealing configuration for temperature.
#[derive(Clone, Debug)]
pub struct TempScheduleConfig {
    pub start: f64,
    pub end: f64,
    pub steps: usize,
}

impl From<TempScheduleConfig> for TemperatureSchedule {
    fn from(value: TempScheduleConfig) -> Self {
        TemperatureSchedule::Linear {
            start: value.start,
            end: value.end,
            steps: value.steps,
        }
    }
}

/// Scale factor translating informational budgets to rotation heuristics.
#[derive(Clone, Debug)]
pub struct InformationToRotation {
    scale: f64,
}

impl InformationToRotation {
    pub fn new(scale: f64) -> Self {
        Self { scale }
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }
}

/// Metadata persisted in span logs.
#[derive(Debug, Clone)]
pub struct LogMetadata {
    pub run_id: String,
    pub timestamp: String,
    pub contract_name: Option<String>,
    pub environment: String,
    pub temperature: f64,
    pub time_step_ms: u64,
    pub accepted_spans: usize,
    pub rejected_spans: usize,
    pub acceptance_rate: f64,
    pub final_potential_energy: f64,
    pub final_gibbs_energy: f64,
    pub informational_efficiency: f64,
    pub total_work: f64,
}

/// Span representation compatible with the CLI replay command.
#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct FoldSpan {
    pub id: String,
    pub delta_theta: f64,
    pub delta_S: f64,
    pub delta_I: f64,
    pub delta_E: f64,
    pub duration_ms: u64,
    pub ghost_flag: bool,
    pub G: f64,
}

impl FoldSpan {
    fn from_outcome(outcome: &folding_core::RotationOutcome) -> Self {
        Self {
            id: outcome.span_record.id.clone(),
            delta_theta: outcome.span_record.delta_theta,
            delta_S: outcome.span_record.delta_entropy,
            delta_I: outcome.span_record.delta_information,
            delta_E: outcome.span_record.delta_energy,
            duration_ms: outcome.span_record.duration.as_millis() as u64,
            ghost_flag: outcome.ghost,
            G: outcome.span_record.gibbs_energy,
        }
    }

    fn to_line(&self) -> String {
        format!(
            "span|id={}|delta_theta={:.6}|delta_S={:.6}|delta_I={:.6}|delta_E={:.6}|duration_ms={}|ghost_flag={}|G={:.6}",
            escape_field(&self.id),
            self.delta_theta,
            self.delta_S,
            self.delta_I,
            self.delta_E,
            self.duration_ms,
            if self.ghost_flag { 1 } else { 0 },
            self.G
        )
    }
}

/// Writes JSONL logs with metadata and span entries.
#[derive(Default)]
pub struct LogLineWriter;

impl LogLineWriter {
    pub fn new() -> Self {
        Self
    }

    pub fn write_report(
        &mut self,
        path: &Path,
        metadata: &LogMetadata,
        report: &ExecutionReport,
    ) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut file = File::create(path)?;
        writeln!(file, "{}", metadata_line(metadata))?;

        for outcome in &report.applied_rotations {
            let span = FoldSpan::from_outcome(outcome);
            writeln!(file, "{}", span.to_line())?;
        }
        for outcome in &report.ghost_rotations {
            let mut span = FoldSpan::from_outcome(outcome);
            span.ghost_flag = true;
            writeln!(file, "{}", span.to_line())?;
        }
        for violation in &report.rejections {
            writeln!(
                file,
                "violation|detail={}",
                escape_field(&format!("{violation:?}"))
            )?;
        }
        Ok(())
    }
}

/// Description of environmental presets used by the CLI.
#[derive(Clone, Debug)]
pub struct EnvironmentPreset {
    pub name: String,
    pub default_temperature: f64,
}

impl EnvironmentPreset {
    pub fn aqueous() -> Self {
        Self {
            name: "aqueous".into(),
            default_temperature: 298.0,
        }
    }

    pub fn cytosol() -> Self {
        Self {
            name: "cytosol".into(),
            default_temperature: 310.0,
        }
    }

    pub fn vacuum() -> Self {
        Self {
            name: "vacuum".into(),
            default_temperature: 295.0,
        }
    }

    pub fn by_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "aqueous" => Some(Self::aqueous()),
            "cytosol" => Some(Self::cytosol()),
            "vacuum" => Some(Self::vacuum()),
            _ => None,
        }
    }
}

/// In-memory preset containing a peptide chain and associated contract.
#[derive(Clone)]
pub struct PresetPack {
    pub chain: PeptideChain,
    pub contract: FoldingContract,
}

pub struct PresetLoader;

impl PresetLoader {
    pub fn load_preset(name: &str) -> Option<PresetPack> {
        if name.eq_ignore_ascii_case("demo") {
            return Self::load_demo();
        }
        Self::load_from_contract(name)
    }

    fn load_demo() -> Option<PresetPack> {
        let contract_path = Path::new("contracts/demo_chain.lll");
        let contract = InputLoader::load_contract(contract_path).ok()?;
        let chain =
            chain_for_contract(&contract).unwrap_or_else(|| PeptideChain::from_sequence("ACDEFGH"));
        Some(PresetPack { chain, contract })
    }

    fn load_from_contract(name: &str) -> Option<PresetPack> {
        let contract_path = Path::new("contracts").join(format!("{name}.lll"));
        if !contract_path.exists() {
            return None;
        }
        let contract = InputLoader::load_contract(&contract_path).ok()?;
        let chain = chain_for_contract(&contract)
            .unwrap_or_else(|| PeptideChain::from_sequence("ACDEFGHIK"));
        Some(PresetPack { chain, contract })
    }
}

fn chain_for_contract(contract: &FoldingContract) -> Option<PeptideChain> {
    let mut max_index: Option<usize> = None;
    for instruction in &contract.instructions {
        if let ContractInstruction::Rotate { residue, .. } = instruction {
            let idx = residue.0;
            max_index = Some(max_index.map(|current| current.max(idx)).unwrap_or(idx));
        }
    }
    let length = max_index? + 1;
    let sequence = generate_sequence(length);
    Some(PeptideChain::from_sequence(&sequence))
}

fn generate_sequence(length: usize) -> String {
    const CANONICAL: &[u8] = b"ACDEFGHIKLMNPQRSTVWY";
    (0..length)
        .map(|idx| CANONICAL[idx % CANONICAL.len()] as char)
        .collect()
}

/// Loads input artifacts from disk.
pub struct InputLoader;

impl InputLoader {
    pub fn load_fasta(path: &Path) -> Result<PeptideChain, String> {
        let contents = fs::read_to_string(path)
            .map_err(|err| format!("failed to read FASTA {}: {err}", path.display()))?;
        let sequence: String = contents
            .lines()
            .filter(|line| !line.starts_with('>'))
            .flat_map(|line| line.chars())
            .filter(|ch| !ch.is_whitespace())
            .collect();
        if sequence.is_empty() {
            return Err("FASTA contained no sequence data".into());
        }
        Ok(PeptideChain::from_sequence(&sequence))
    }

    pub fn load_contract(path: &Path) -> Result<FoldingContract, String> {
        let contents = fs::read_to_string(path)
            .map_err(|err| format!("failed to read contract {}: {err}", path.display()))?;
        let lines: Vec<&str> = contents.lines().collect();
        Ok(FoldingContract::from_lines(&lines))
    }
}

/// CLI orchestrator bridging configuration, runtime, and logging.
pub struct CommandShell {
    writer: LogLineWriter,
    _info_to_rotation: InformationToRotation,
    config: ShellConfig,
    last_log_path: Option<PathBuf>,
    last_diamond_path: Option<PathBuf>,
    contract_label: Option<String>,
}

impl CommandShell {
    pub fn new(
        writer: LogLineWriter,
        info_to_rotation: InformationToRotation,
        config: ShellConfig,
    ) -> Self {
        Self {
            writer,
            _info_to_rotation: info_to_rotation,
            config,
            last_log_path: None,
            last_diamond_path: None,
            contract_label: None,
        }
    }

    pub fn set_contract_label(&mut self, label: Option<String>) {
        self.contract_label = label;
    }

    pub fn config(&self) -> &ShellConfig {
        &self.config
    }

    pub fn last_log_path(&self) -> Option<&PathBuf> {
        self.last_log_path.as_ref()
    }

    pub fn last_diamond_path(&self) -> Option<&PathBuf> {
        self.last_diamond_path.as_ref()
    }

    pub fn run_contract(
        &mut self,
        chain: PeptideChain,
        contract: FoldingContract,
    ) -> ExecutionReport {
        let mut builder = FoldingEngineBuilder::new()
            .with_chain(chain)
            .with_energy_model(EnergyModel::default())
            .with_temperature(self.config.temperature)
            .with_ruleset(folding_core::Ruleset::default());

        if let Some(seed) = self.config.rng_seed {
            builder = builder.with_rng_seed(seed);
        }
        if let Some(schedule) = self.config.temp_schedule.clone() {
            builder = builder.with_temperature_schedule(schedule.into());
        }
        let mut engine = builder.build();
        let report = engine.execute_contract(&contract);

        let metrics = FoldingMetrics::from_report(&report);
        let run_id = generate_run_id();
        let metadata = self.build_metadata(&report, &metrics, &run_id);
        let log_path = self.resolve_log_path(&run_id);
        if let Err(err) = self.writer.write_report(&log_path, &metadata, &report) {
            eprintln!("failed to write span log {}: {err}", log_path.display());
            self.last_log_path = None;
        } else {
            self.last_log_path = Some(log_path);
        }

        self.last_diamond_path = None;
        report
    }

    fn resolve_log_path(&self, run_id: &str) -> PathBuf {
        if let Some(custom) = self.config.log_path.as_ref() {
            return custom.clone();
        }
        let label = self.contract_label.as_deref().unwrap_or("fold");
        Path::new("logs").join(format!("{}_{}.log", label, run_id))
    }

    fn build_metadata(
        &self,
        report: &ExecutionReport,
        metrics: &FoldingMetrics,
        run_id: &str,
    ) -> LogMetadata {
        let stats: &MetropolisStats = &report.metropolis_stats;
        let timestamp = current_timestamp();
        let accepted = report.applied_rotations.len();
        let rejected = report.rejections.len();
        let total_entropy = report.trajectory.total_entropy();
        let final_gibbs =
            report.final_energy.total_potential - self.config.temperature * total_entropy;
        let efficiency = if (metrics.total_entropy + metrics.ghost_entropy).abs() < f64::EPSILON {
            0.0
        } else {
            metrics.total_entropy / (metrics.total_entropy + metrics.ghost_entropy)
        };
        LogMetadata {
            run_id: run_id.to_string(),
            timestamp,
            contract_name: self.contract_label.clone(),
            environment: self.config.environment.clone(),
            temperature: self.config.temperature,
            time_step_ms: self.config.time_step_ms,
            accepted_spans: accepted,
            rejected_spans: rejected,
            acceptance_rate: stats.acceptance_rate(),
            final_potential_energy: report.final_energy.total_potential,
            final_gibbs_energy: final_gibbs,
            informational_efficiency: efficiency,
            total_work: compute_total_work(report),
        }
    }
}

fn compute_total_work(report: &ExecutionReport) -> f64 {
    report
        .applied_rotations
        .iter()
        .map(|outcome| {
            let duration = outcome.span_record.duration.as_secs_f64();
            outcome.span_record.delta_energy.abs() * duration
        })
        .sum()
}

fn metadata_line(metadata: &LogMetadata) -> String {
    format!(
        "metadata|run_id={}|timestamp={}|contract_name={}|environment={}|temperature={:.6}|time_step_ms={}|accepted_spans={}|rejected_spans={}|acceptance_rate={:.6}|final_potential_energy={:.6}|final_gibbs_energy={:.6}|informational_efficiency={:.6}|total_work={:.6}",
        escape_field(&metadata.run_id),
        escape_field(&metadata.timestamp),
        escape_field(metadata.contract_name.as_deref().unwrap_or("")),
        escape_field(&metadata.environment),
        metadata.temperature,
        metadata.time_step_ms,
        metadata.accepted_spans,
        metadata.rejected_spans,
        metadata.acceptance_rate,
        metadata.final_potential_energy,
        metadata.final_gibbs_energy,
        metadata.informational_efficiency,
        metadata.total_work
    )
}

fn escape_field(value: &str) -> String {
    value.replace('|', "_").replace('=', "_")
}

fn generate_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

fn current_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:09}", duration.as_secs(), duration.subsec_nanos())
}

/// Convenience alias re-exported for CLI consumers.
pub type FoldSpanRecord = FoldSpan;

/// Utilities exposed for integration tests.
pub fn compute_sequence_length(contract: &FoldingContract) -> usize {
    chain_for_contract(contract)
        .map(|chain| chain.residues().len())
        .unwrap_or(0)
}
