use std::path::{Path, PathBuf};

/// Parsed representation of `logline fold` CLI arguments.
#[derive(Debug, Clone)]
pub struct FoldCommand {
    pub input: PathBuf,
    pub output: PathBuf,
    pub engine: String,
    pub rollback: bool,
    pub contract_path: PathBuf,
}

impl FoldCommand {
    /// Parses the `logline fold` subcommand arguments.
    ///
    /// The expected syntax is:
    /// `logline fold <INPUT> [--output <OUTPUT>] [--engine <ENGINE>] [--contract <CONTRACT>] [--rollback]`
    pub fn parse(args: &[String]) -> Result<Self, String> {
        if args.is_empty() {
            return Err("missing input sequence (FASTA or JSON)".into());
        }

        let input = PathBuf::from(&args[0]);
        let mut output: Option<PathBuf> = None;
        let mut engine: Option<String> = None;
        let mut contract: Option<PathBuf> = None;
        let mut rollback = false;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--output" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--output expects a path".to_string())?;
                    output = Some(PathBuf::from(value));
                }
                "--engine" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--engine expects a value".to_string())?;
                    engine = Some(value.clone());
                }
                "--contract" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--contract expects a path".to_string())?;
                    contract = Some(PathBuf::from(value));
                }
                "--rollback" => {
                    rollback = true;
                }
                "--no-rollback" => {
                    rollback = false;
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown fold argument: {other}"));
                }
                other => {
                    return Err(format!(
                        "unexpected positional argument '{other}'. Expected only the input path."
                    ));
                }
            }
            index += 1;
        }

        let output = output.unwrap_or_else(|| default_output_path(&input));
        let contract_path = contract.unwrap_or_else(|| default_contract_path(&output));

        Ok(Self {
            input,
            output,
            engine: engine.unwrap_or_else(|| "logline".to_string()),
            rollback,
            contract_path,
        })
    }
}

fn default_output_path(input: &Path) -> PathBuf {
    let mut path = PathBuf::from(input);
    path.set_extension("pdb");
    path
}

fn default_contract_path(output: &Path) -> PathBuf {
    let mut path = PathBuf::from(output);
    path.set_extension("lll");
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_arguments() {
        let args = vec!["input.fasta".to_string()];
        let cmd = FoldCommand::parse(&args).unwrap();
        assert_eq!(cmd.input, PathBuf::from("input.fasta"));
        assert_eq!(cmd.output, PathBuf::from("input.pdb"));
        assert_eq!(cmd.contract_path, PathBuf::from("input.lll"));
        assert_eq!(cmd.engine, "logline");
        assert!(!cmd.rollback);
    }

    #[test]
    fn parses_all_flags() {
        let args = vec![
            "input.fa".into(),
            "--output".into(),
            "result.pdb".into(),
            "--engine".into(),
            "toy".into(),
            "--contract".into(),
            "workflow.lll".into(),
            "--rollback".into(),
        ];
        let cmd = FoldCommand::parse(&args).unwrap();
        assert_eq!(cmd.output, PathBuf::from("result.pdb"));
        assert_eq!(cmd.contract_path, PathBuf::from("workflow.lll"));
        assert_eq!(cmd.engine, "toy");
        assert!(cmd.rollback);
    }

    #[test]
    fn rejects_unknown_flags() {
        let args = vec!["input.fa".into(), "--weird".into()];
        assert!(FoldCommand::parse(&args).is_err());
    }
}
