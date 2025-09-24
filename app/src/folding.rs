use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use folding_molecule::PeptideChain;

use crate::cli::FoldCommand;
use crate::protein::ProteinSequence;

/// Result bundle produced after executing a folding run.
pub struct FoldingArtifacts {
    pub sequence: ProteinSequence,
    pub chain: PeptideChain,
    pub contract: String,
    pub embeddings: Option<Vec<f32>>,
}

/// Entry point for the `logline fold` CLI.
pub fn run_fold(command: &FoldCommand) -> Result<FoldingArtifacts, String> {
    if command.engine.to_ascii_lowercase() != "logline" {
        return Err(format!(
            "unsupported engine '{}'. Only 'logline' is available in v0.1.",
            command.engine
        ));
    }

    let sequence = crate::protein::load_sequence(&command.input)?;
    let embeddings = try_fetch_torch_embeddings(&sequence.sequence);
    let mut chain = sequence.to_chain();
    refine_geometry(&mut chain);

    let contract = render_contract(&command.input, &command.output, command.rollback);

    Ok(FoldingArtifacts {
        sequence,
        chain,
        contract,
        embeddings,
    })
}

fn refine_geometry(chain: &mut PeptideChain) {
    if chain.is_empty() {
        return;
    }
    let pitch = 1.5_f64;
    let radius = 3.2_f64;
    for (index, residue) in chain.residues_mut().iter_mut().enumerate() {
        let idx = index as f64;
        let angle = idx * 2.0 * std::f64::consts::PI / 3.6; // Rough alpha-helix cadence
        let base_z = idx * pitch;
        let radial_offset = ((index % 5) as f64) * 0.1; // break symmetry slightly
        let x = (radius + radial_offset) * angle.cos();
        let y = (radius + radial_offset) * angle.sin();
        let z = base_z;
        residue.set_position([x, y, z]);
        residue.phi = angle.to_degrees() % 360.0;
        residue.psi = (angle + std::f64::consts::FRAC_PI_2).to_degrees() % 360.0;
    }
}

/// Attempts to fetch embeddings by delegating to a Python + PyTorch helper.
fn try_fetch_torch_embeddings(sequence: &str) -> Option<Vec<f32>> {
    let python = env::var("PYTHON_TORCH_BIN").unwrap_or_else(|_| "python3".to_string());
    let helper = python_helper_path();
    if !helper.exists() {
        // No helper script available; nothing to do.
        return None;
    }

    let output = Command::new(python)
        .arg(&helper)
        .arg(sequence)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut values = Vec::new();
    for token in stdout.split_whitespace() {
        if let Ok(value) = token.parse::<f32>() {
            values.push(value);
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn python_helper_path() -> PathBuf {
    Path::new(
        &env::var("LOGLINE_TORCH_HELPER").unwrap_or_else(|_| "scripts/torch_embeddings.py".into()),
    )
    .into()
}

pub fn persist_contract(contract: &str, path: &Path) -> Result<(), String> {
    fs::write(path, contract)
        .map_err(|err| format!("failed to write contract {}: {err}", path.display()))
}

fn render_contract(input: &Path, output: &Path, rollback: bool) -> String {
    format!(
        "span:type=protein_folding\nid: logline_fold_run\nsteps:\n  - load: \"{}\"\n  - fold: \"logline_encoder_v1\"\n  - export: \"{}\"\nrollback: {}\n",
        input.display(),
        output.display(),
        if rollback { "true" } else { "false" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_contract_matches_manifesto() {
        let contract = render_contract(Path::new("input.fasta"), Path::new("output.pdb"), true);
        assert!(contract.contains("span:type=protein_folding"));
        assert!(contract.contains("rollback: true"));
    }

    #[test]
    fn refine_geometry_adjusts_positions() {
        let sequence = ProteinSequence {
            identifier: None,
            sequence: "ACDEFG".into(),
        };
        let mut chain = sequence.to_chain();
        let original = chain.residues()[0].position();
        refine_geometry(&mut chain);
        assert_ne!(chain.residues()[0].position()[0], original[0]);
    }
}
