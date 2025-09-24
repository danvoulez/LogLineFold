use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use folding_interface::{
    CommandShell, EnvironmentPreset, FoldSpan, InformationToRotation, InputLoader, LogLineWriter,
    LogMetadata, PresetLoader, ShellConfig, TempScheduleConfig,
};
use folding_sim::{FoldingMetrics, TrajectoryVisualizer};
use serde_json::Value;

struct CliOptions {
    preset: Option<String>,
    fasta: Option<PathBuf>,
    contract: Option<PathBuf>,
    temperature: Option<f64>,
    time_step_ms: Option<u64>,
    rng_seed: Option<u64>,
    log_path: Option<PathBuf>,
    replay: Option<PathBuf>,
    info_scale: f64,
    environment: Option<String>,
    diamond_threshold: Option<f64>,
    diamond_dir: Option<PathBuf>,
    show_ghosts: bool,
    temp_schedule: Option<(f64, f64, usize)>,
}

impl CliOptions {
    fn parse() -> Result<Self, String> {
        let args: Vec<String> = env::args().skip(1).collect();
        let mut options = Self {
            preset: None,
            fasta: None,
            contract: None,
            temperature: None,
            time_step_ms: None,
            rng_seed: None,
            log_path: None,
            replay: None,
            info_scale: 0.01,
            environment: None,
            diamond_threshold: None,
            diamond_dir: None,
            show_ghosts: false,
            temp_schedule: None,
        };

        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            let mut next = || {
                i += 1;
                args.get(i)
                    .ok_or_else(|| format!("missing value for argument '{}'.", arg))
                    .cloned()
            };
            match arg.as_str() {
                "--preset" => options.preset = Some(next()?),
                "--fasta" => options.fasta = Some(PathBuf::from(next()?)),
                "--contract" => options.contract = Some(PathBuf::from(next()?)),
                "--temp" | "--temperature" => {
                    options.temperature = Some(
                        next()?
                            .parse()
                            .map_err(|_| "invalid temperature".to_string())?,
                    )
                }
                "--dt" | "--time-step" => {
                    options.time_step_ms = Some(
                        next()?
                            .parse()
                            .map_err(|_| "invalid time step".to_string())?,
                    )
                }
                "--seed" => {
                    options.rng_seed =
                        Some(next()?.parse().map_err(|_| "invalid seed".to_string())?)
                }
                "--log" => options.log_path = Some(PathBuf::from(next()?)),
                "--replay" => options.replay = Some(PathBuf::from(next()?)),
                "--info-scale" => {
                    options.info_scale = next()?
                        .parse()
                        .map_err(|_| "invalid info scale".to_string())?
                }
                "--env" | "--environment" => options.environment = Some(next()?),
                "--diamond-threshold" => {
                    options.diamond_threshold = Some(
                        next()?
                            .parse()
                            .map_err(|_| "invalid diamond threshold".to_string())?,
                    )
                }
                "--diamond-dir" => options.diamond_dir = Some(PathBuf::from(next()?)),
                "--anneal" => {
                    let raw = next()?;
                    let parts: Vec<&str> = raw.split(':').collect();
                    if parts.len() != 3 {
                        return Err("--anneal expects start:end:steps".to_string());
                    }
                    let start = parts[0]
                        .parse()
                        .map_err(|_| "invalid anneal start".to_string())?;
                    let end = parts[1]
                        .parse()
                        .map_err(|_| "invalid anneal end".to_string())?;
                    let steps = parts[2]
                        .parse()
                        .map_err(|_| "invalid anneal steps".to_string())?;
                    options.temp_schedule = Some((start, end, steps));
                }
                "--ghosts" => options.show_ghosts = true,
                other if other.starts_with('-') => {
                    return Err(format!("unknown argument: {}", other));
                }
                other => {
                    options.preset = Some(other.to_string());
                }
            }
            i += 1;
        }

        Ok(options)
    }
}

fn run_replay(path: &Path, show_ghosts: bool) -> Result<(), String> {
    let file =
        File::open(path).map_err(|err| format!("failed to open log {}: {err}", path.display()))?;
    let mut lines = BufReader::new(file).lines();

    let metadata_line = lines
        .next()
        .ok_or_else(|| "log file is empty".to_string())?
        .map_err(|err| err.to_string())?;
    let metadata: LogMetadata = serde_json::from_str(&metadata_line)
        .map_err(|err| format!("failed to parse metadata: {err}"))?;

    let mut spans: Vec<FoldSpan> = Vec::new();
    let mut violation_details = Vec::new();

    for line in lines {
        let line = line.map_err(|err| err.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line)
            .map_err(|err| format!("failed to parse log line: {err}"))?;
        if value.get("type").and_then(Value::as_str) == Some("violation") {
            violation_details.push(
                value
                    .get("detail")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown violation")
                    .to_string(),
            );
            continue;
        }
        let mut span: FoldSpan =
            serde_json::from_value(value).map_err(|err| format!("failed to decode span: {err}"))?;
        if span.delta_E > 0.0 {
            span.ghost_flag = true;
        }
        spans.push(span);
    }

    let applied = spans.iter().filter(|s| !s.ghost_flag).count();
    let ghost = spans.len().saturating_sub(applied);
    let total_work: f64 = spans
        .iter()
        .filter(|s| !s.ghost_flag)
        .map(|s| s.delta_E.abs() * (metadata.time_step_ms as f64 / 1000.0))
        .sum();

    println!("Replay summary for {}", path.display());
    println!(
        "  Contract: {}",
        metadata.contract_name.unwrap_or_else(|| "unknown".into())
    );
    println!("  Environment: {}", metadata.environment);
    println!("  Temperature: {:.2} K", metadata.temperature);
    println!("  Accepted spans: {}", metadata.accepted_spans);
    println!("  Rejected spans: {}", metadata.rejected_spans);
    println!(
        "  Acceptance rate: {:.2}%",
        metadata.acceptance_rate * 100.0
    );
    println!("  Applied spans: {}", applied);
    println!("  Ghost spans: {}", ghost);
    println!("  Violations recorded: {}", violation_details.len());
    println!(
        "  Final potential energy: {:.4}",
        metadata.final_potential_energy
    );
    println!("  Final Gibbs energy: {:.4}", metadata.final_gibbs_energy);
    println!(
        "  Informational efficiency η: {:.6}",
        metadata.informational_efficiency
    );
    println!(
        "  Total work (approx): {:.6}",
        total_work.max(metadata.total_work)
    );

    if show_ghosts {
        println!("\nSpans:");
        for (idx, span) in spans.iter().enumerate() {
            let status = if span.ghost_flag { "GHOST" } else { "ACCEPT" };
            println!(
                "  Step {:04} [{}] Δθ={:.4} ΔE={:.6} ΔS={:.6} G={:.6}",
                idx + 1,
                status,
                span.delta_theta,
                span.delta_E,
                span.delta_S,
                span.G
            );
        }
    }

    if !violation_details.is_empty() {
        println!("\nViolations:");
        for detail in violation_details {
            println!("  - {}", detail);
        }
    }

    Ok(())
}

fn main() {
    let opts = match CliOptions::parse() {
        Ok(o) => o,
        Err(err) => {
            eprintln!("argument error: {err}");
            return;
        }
    };

    if let Some(path) = opts.replay.as_ref() {
        if let Err(err) = run_replay(path, opts.show_ghosts) {
            eprintln!("replay failed: {err}");
        }
        return;
    }

    let mut chain = None;
    let mut contract = None;
    let mut label = opts.preset.clone();

    if let Some(preset) = opts.preset.as_deref() {
        if let Some(preset_pack) = PresetLoader::load_preset(preset) {
            chain = Some(preset_pack.chain);
            contract = Some(preset_pack.contract);
        } else {
            eprintln!("preset '{}' not found; falling back to demo", preset);
            label = Some("demo".into());
        }
    }

    if chain.is_none() || contract.is_none() {
        if let Some(preset_pack) = PresetLoader::load_preset(label.as_deref().unwrap_or("demo")) {
            chain.get_or_insert(preset_pack.chain.clone());
            contract.get_or_insert(preset_pack.contract.clone());
            if label.is_none() {
                label = Some("demo".into());
            }
        }
    }

    if let Some(fasta_path) = opts.fasta.as_ref() {
        match InputLoader::load_fasta(fasta_path) {
            Ok(loaded_chain) => chain = Some(loaded_chain),
            Err(err) => {
                eprintln!("failed to load FASTA {}: {err}", fasta_path.display());
                return;
            }
        }
    }

    if let Some(contract_path) = opts.contract.as_ref() {
        match InputLoader::load_contract(contract_path) {
            Ok(loaded_contract) => contract = Some(loaded_contract),
            Err(err) => {
                eprintln!("failed to load contract {}: {err}", contract_path.display());
                return;
            }
        }
        if label.is_none() {
            label = contract_path
                .file_stem()
                .map(|s| s.to_string_lossy().into());
        }
    }

    let Some(chain) = chain else {
        eprintln!("no chain available after parsing inputs");
        return;
    };
    let Some(contract) = contract else {
        eprintln!("no contract available after parsing inputs");
        return;
    };

    let environment = opts
        .environment
        .clone()
        .unwrap_or_else(|| "aqueous".to_string());
    let env_preset =
        EnvironmentPreset::by_name(&environment).unwrap_or_else(EnvironmentPreset::aqueous);
    let temperature = opts.temperature.unwrap_or(env_preset.default_temperature);

    let config = ShellConfig {
        temperature,
        time_step_ms: opts.time_step_ms.unwrap_or(1),
        rng_seed: opts.rng_seed,
        log_path: opts.log_path.clone(),
        environment,
        diamond_threshold: opts.diamond_threshold,
        diamond_path: opts.diamond_dir.clone(),
        temp_schedule: opts
            .temp_schedule
            .map(|(start, end, steps)| TempScheduleConfig { start, end, steps }),
    };

    let mut shell = CommandShell::new(
        LogLineWriter::new(),
        InformationToRotation::new(opts.info_scale),
        config,
    );
    shell.set_contract_label(label.clone());

    let shell_report = shell.run_contract(chain, contract);
    let metrics = FoldingMetrics::from_report(&shell_report);
    let trajectory_json = TrajectoryVisualizer::to_json(&shell_report.trajectory);

    let total_steps = shell_report.applied_rotations.len() + shell_report.ghost_rotations.len();
    let convergence_threshold = 1e-3;
    let converged_at = shell_report
        .applied_rotations
        .iter()
        .rposition(|outcome| outcome.span_record.delta_energy.abs() > convergence_threshold)
        .map(|idx| idx + 1)
        .unwrap_or(total_steps);
    let final_gibbs = shell_report.final_energy.total_potential
        - shell.config().temperature * shell_report.trajectory.total_entropy();
    let efficiency = if metrics.total_entropy + metrics.ghost_entropy <= f64::EPSILON {
        0.0
    } else {
        (metrics.total_entropy / (metrics.total_entropy + metrics.ghost_entropy)) * 100.0
    };

    println!("Folding run complete.");
    println!("Total steps: {}", total_steps);
    println!("Converged at: {}", converged_at);
    println!("Final G: {:.6}", final_gibbs);
    println!("Informational Efficiency: {:.1}%", efficiency);
    let stats = &shell_report.metropolis_stats;
    println!(
        "Accepted / Rejected: {} / {} ({:.1}% acceptance)",
        stats.accepted,
        stats.rejected,
        stats.acceptance_rate() * 100.0
    );

    if let Some(log_path) = shell.last_log_path() {
        println!("Spans persisted at: {}", log_path.display());
    } else {
        println!("Spans persisted at: <not written>");
    }

    if let Some(diamond_path) = shell.last_diamond_path() {
        println!("Diamonds persisted at: {}", diamond_path.display());
    } else {
        println!("Diamonds persisted at: logs/diamonds.json (no entry)");
    }

    println!("Trajectory snapshot: {}", trajectory_json);
}
