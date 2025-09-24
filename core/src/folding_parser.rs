use std::collections::HashMap;

use folding_molecule::ResidueId;

/// Core instruction set for `.lll` folding contracts.
#[derive(Debug, Clone)]
pub enum ContractInstruction {
    Rotate {
        residue: ResidueId,
        angle_degrees: f64,
        duration_ms: u64,
    },
    ClashCheck,
    Commit,
    Rollback,
    GhostMode(bool),
    SpanAlias(String),
    DefineDomain {
        name: Option<String>,
        start: ResidueId,
        end: ResidueId,
    },
    RequireChaperone {
        chaperone: String,
        span: Option<String>,
    },
    AddModification {
        modification: String,
        residue: ResidueId,
    },
    SetPhysicsLevel(PhysicsLevel),
    SetSpanPhysics(PhysicsSpanMode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsLevel {
    Toy,
    Coarse,
    Gb,
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsSpanMode {
    Toy,
    Physics,
}

/// Parsed folding contract ready for execution.
#[derive(Debug, Clone, Default)]
pub struct FoldingContract {
    pub instructions: Vec<ContractInstruction>,
}

impl FoldingContract {
    pub fn new(instructions: Vec<ContractInstruction>) -> Self {
        Self { instructions }
    }

    pub fn from_lines(lines: &[&str]) -> Self {
        let instructions = lines
            .iter()
            .filter_map(|line| parse_line(line))
            .flatten()
            .collect();
        Self { instructions }
    }
}

fn parse_line(raw_line: &str) -> Option<Vec<ContractInstruction>> {
    let line_without_comments = raw_line.split('#').next().map(str::trim).unwrap_or("");

    if line_without_comments.is_empty() {
        return None;
    }

    let mut tokens = tokenize(line_without_comments);
    if tokens.is_empty() {
        return None;
    }

    let command = tokens.remove(0).to_lowercase();
    let mut instructions = Vec::new();

    match command.as_str() {
        "rotate" => {
            if let Some(instr) = parse_rotate(tokens) {
                instructions.push(instr);
            }
        }
        "clash_check" | "clash" => instructions.push(ContractInstruction::ClashCheck),
        "commit" => instructions.push(ContractInstruction::Commit),
        "rollback" | "revert" => instructions.push(ContractInstruction::Rollback),
        "ghost" => {
            if let Some(state) = tokens.first() {
                let flag = matches!(state.as_str().to_lowercase().as_str(), "on" | "true" | "1");
                instructions.push(ContractInstruction::GhostMode(flag));
            }
        }
        "span_alias" | "alias" | "label" => {
            let alias = tokens.join(" ");
            let alias = alias.trim_matches(|c| c == '"' || c == '\'');
            if !alias.is_empty() {
                instructions.push(ContractInstruction::SpanAlias(alias.to_owned()));
            }
        }
        "define_domain" | "domain" => {
            if let Some(instr) = parse_define_domain(tokens) {
                instructions.push(instr);
            }
        }
        "require_chaperone" | "chaperone" => {
            if let Some(instr) = parse_require_chaperone(tokens) {
                instructions.push(instr);
            }
        }
        "add_modification" | "modification" | "modify" => {
            if let Some(instr) = parse_add_modification(tokens) {
                instructions.push(instr);
            }
        }
        "set_physics_level" | "physics_level" => {
            if let Some(instr) = parse_set_physics_level(tokens) {
                instructions.push(instr);
            }
        }
        "physics_span" | "set_span_physics" => {
            if let Some(instr) = parse_set_span_physics(tokens) {
                instructions.push(instr);
            }
        }
        _ => return None,
    }

    if instructions.is_empty() {
        None
    } else {
        Some(instructions)
    }
}

fn parse_rotate(tokens: Vec<String>) -> Option<ContractInstruction> {
    if tokens.is_empty() {
        return None;
    }

    let (residue, angle, duration) = if tokens
        .iter()
        .any(|token| token.contains('=') || token.contains(':'))
    {
        parse_rotate_keyed(tokens)?
    } else {
        parse_rotate_positional(tokens)?
    };

    Some(ContractInstruction::Rotate {
        residue: ResidueId(residue),
        angle_degrees: angle,
        duration_ms: duration,
    })
}

fn parse_define_domain(tokens: Vec<String>) -> Option<ContractInstruction> {
    if tokens.is_empty() {
        return None;
    }

    let mut name: Option<String> = None;
    let range_value = if tokens.len() == 1 {
        tokens[0].clone()
    } else {
        let first = &tokens[0];
        if is_range_token(first) {
            first.clone()
        } else {
            name = Some(first.clone());
            tokens.get(1).cloned().unwrap_or_else(|| first.clone())
        }
    };

    let (start, end) = parse_range(&range_value)?;

    Some(ContractInstruction::DefineDomain {
        name,
        start: ResidueId(start),
        end: ResidueId(end),
    })
}

fn parse_require_chaperone(tokens: Vec<String>) -> Option<ContractInstruction> {
    if tokens.is_empty() {
        return None;
    }
    let mut chaperone_parts = Vec::new();
    let mut span_parts = Vec::new();
    let mut in_span = false;

    for token in tokens {
        let lower = token.to_lowercase();
        if lower == "for" || lower == "span" {
            in_span = true;
            continue;
        }
        if in_span {
            span_parts.push(token);
        } else {
            chaperone_parts.push(token);
        }
    }

    if chaperone_parts.is_empty() {
        return None;
    }
    let chaperone = chaperone_parts.join(" ");
    let span = if span_parts.is_empty() {
        None
    } else {
        Some(span_parts.join(" "))
    };

    Some(ContractInstruction::RequireChaperone { chaperone, span })
}

fn parse_add_modification(tokens: Vec<String>) -> Option<ContractInstruction> {
    if tokens.is_empty() {
        return None;
    }

    let mut modification_parts = Vec::new();
    let mut residue_token: Option<String> = None;
    let mut iter = tokens.into_iter();
    while let Some(token) = iter.next() {
        let lower = token.to_lowercase();
        if lower == "at" || lower == "on" {
            residue_token = iter.next();
            break;
        } else {
            modification_parts.push(token);
        }
    }
    let modification = modification_parts.join(" ");
    if modification.is_empty() {
        return None;
    }
    let residue_token = residue_token.unwrap_or_default();
    let residue_index = parse_residue(&residue_token)?;

    Some(ContractInstruction::AddModification {
        modification,
        residue: ResidueId(residue_index),
    })
}

fn parse_set_physics_level(tokens: Vec<String>) -> Option<ContractInstruction> {
    if tokens.is_empty() {
        return None;
    }

    let raw = tokens[0].clone();
    let value = if raw.contains(['=', ':']) {
        split_key_value(&raw)?.1
    } else {
        raw
    };

    let level = match value.to_lowercase().as_str() {
        "toy" | "none" | "off" => PhysicsLevel::Toy,
        "coarse" | "cg" => PhysicsLevel::Coarse,
        "gb" | "implicit" => PhysicsLevel::Gb,
        "full" | "explicit" => PhysicsLevel::Full,
        _ => return None,
    };

    Some(ContractInstruction::SetPhysicsLevel(level))
}

fn parse_set_span_physics(tokens: Vec<String>) -> Option<ContractInstruction> {
    if tokens.is_empty() {
        return None;
    }

    let raw = tokens[0].clone();
    let value = if raw.contains(['=', ':']) {
        split_key_value(&raw)?.1
    } else {
        raw
    };

    let mode = match value.to_lowercase().as_str() {
        "toy" | "off" | "none" => PhysicsSpanMode::Toy,
        "physics" | "on" | "gb" | "full" => PhysicsSpanMode::Physics,
        _ => return None,
    };

    Some(ContractInstruction::SetSpanPhysics(mode))
}

fn parse_range(token: &str) -> Option<(usize, usize)> {
    let cleaned = token.trim_matches(|c: char| c == '[' || c == ']' || c.is_whitespace());
    let mut parts = cleaned.split(|c| c == '-' || c == ',');
    let start = parts.next()?.trim();
    let end = parts.next()?.trim();
    let start_id = parse_residue(start)?;
    let end_id = parse_residue(end)?;
    if start_id <= end_id {
        Some((start_id, end_id))
    } else {
        Some((end_id, start_id))
    }
}

fn is_range_token(token: &str) -> bool {
    token.contains('-') && token.chars().any(|c| c.is_ascii_digit())
}

fn parse_rotate_positional(tokens: Vec<String>) -> Option<(usize, f64, u64)> {
    let residue_token = tokens.get(0)?.as_str();
    let angle_token = tokens.get(1)?.as_str();
    let duration_token = tokens.get(2).map(|s| s.as_str());

    let residue = parse_residue(residue_token)?;
    let angle = parse_angle(angle_token)?;
    let duration = duration_token.and_then(parse_duration).unwrap_or(1);

    Some((residue, angle, duration))
}

fn parse_rotate_keyed(tokens: Vec<String>) -> Option<(usize, f64, u64)> {
    let mut key_values: HashMap<String, String> = HashMap::new();
    let mut index = 0;

    while index < tokens.len() {
        let token = tokens[index].clone();
        let cleaned = token.trim_matches(|c| matches!(c, ',' | ';'));
        if cleaned.is_empty() {
            index += 1;
            continue;
        }

        if let Some((key, value)) = split_key_value(cleaned) {
            if !value.is_empty() {
                key_values.insert(key, value);
            } else if index + 1 < tokens.len() {
                let next = tokens[index + 1].trim_matches(|c| matches!(c, ',' | ';'));
                key_values.insert(key, next.to_string());
                index += 1;
            }
        } else {
            // Fallback for positional tokens sprinkled into keyed syntax.
            key_values
                .entry("positional".into())
                .and_modify(|existing| {
                    existing.push(' ');
                    existing.push_str(cleaned);
                })
                .or_insert_with(|| cleaned.to_string());
        }

        index += 1;
    }

    // Support hybrid definitions like "residue=1" or "residue 1" captured above.
    if let Some(positional) = key_values.remove("positional") {
        let mut positional_tokens: Vec<String> = positional
            .split_whitespace()
            .map(|tok| tok.to_string())
            .collect();
        if !positional_tokens.is_empty() {
            key_values
                .entry("residue".into())
                .or_insert_with(|| positional_tokens.remove(0));
        }
        if !positional_tokens.is_empty() {
            key_values
                .entry("angle".into())
                .or_insert_with(|| positional_tokens.remove(0));
        }
        if !positional_tokens.is_empty() {
            key_values
                .entry("duration".into())
                .or_insert_with(|| positional_tokens.remove(0));
        }
    }

    let residue = key_values
        .get("residue")
        .or_else(|| key_values.get("res"))
        .or_else(|| key_values.get("id"))
        .and_then(|value| parse_residue(value))?;

    let angle = key_values
        .get("angle")
        .or_else(|| key_values.get("theta"))
        .or_else(|| key_values.get("deg"))
        .and_then(|value| parse_angle(value))?;

    let duration = key_values
        .get("duration")
        .or_else(|| key_values.get("time"))
        .or_else(|| key_values.get("ms"))
        .and_then(|value| parse_duration(value))
        .unwrap_or(1);

    Some((residue, angle, duration))
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '\0';

    for ch in input.chars() {
        if in_quotes {
            if ch == quote_char {
                tokens.push(current.clone());
                current.clear();
                in_quotes = false;
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '"' | '\'' => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                in_quotes = true;
                quote_char = ch;
            }
            '{' | '}' | '(' | ')' => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            ',' | ';' => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn split_key_value(token: &str) -> Option<(String, String)> {
    let delimiter_index = token.find(['=', ':'])?;
    let key = token[..delimiter_index]
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');
    let value = token[delimiter_index + 1..]
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');

    Some((key.to_lowercase(), value.to_string()))
}

fn parse_residue(token: &str) -> Option<usize> {
    let cleaned = token
        .trim_matches(|c: char| !c.is_ascii_alphanumeric())
        .trim_start_matches(|c: char| c.is_alphabetic() || c == '_')
        .replace('_', "");
    cleaned.parse::<usize>().ok()
}

fn parse_angle(token: &str) -> Option<f64> {
    let cleaned = extract_numeric(token, |c| matches!(c, '.' | '-' | '+'))?;
    cleaned.parse::<f64>().ok()
}

fn parse_duration(token: &str) -> Option<u64> {
    let cleaned = extract_numeric(token, |_| false)?;
    cleaned.parse::<u64>().ok()
}

fn extract_numeric<F>(token: &str, extra: F) -> Option<String>
where
    F: Fn(char) -> bool,
{
    let mut numeric = String::new();
    for ch in token.chars() {
        if ch.is_ascii_digit() || extra(ch) {
            numeric.push(ch);
        }
    }
    if numeric.is_empty() {
        None
    } else {
        Some(numeric)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_instructions() {
        let lines = [
            "rotate 0 90.0 2",
            "clash_check",
            "commit",
            "rollback",
            "ghost on",
            "ghost off",
            "span_alias alpha helix",
        ];
        let contract = FoldingContract::from_lines(&lines);
        assert_eq!(contract.instructions.len(), lines.len());
        assert!(matches!(
            contract.instructions[0],
            ContractInstruction::Rotate { .. }
        ));
        assert!(matches!(
            contract.instructions[4],
            ContractInstruction::GhostMode(true)
        ));
        assert!(matches!(
            contract.instructions[5],
            ContractInstruction::GhostMode(false)
        ));
        match &contract.instructions[6] {
            ContractInstruction::SpanAlias(alias) => assert_eq!(alias, "alpha helix"),
            other => panic!("unexpected instruction: {other:?}"),
        }
    }

    #[test]
    fn parses_key_value_rotation_syntax() {
        let lines = ["rotate residue=4 angle=45deg duration=5ms"];
        let contract = FoldingContract::from_lines(&lines);
        assert_eq!(contract.instructions.len(), 1);
        match &contract.instructions[0] {
            ContractInstruction::Rotate {
                residue,
                angle_degrees,
                duration_ms,
            } => {
                assert_eq!(residue.0, 4);
                assert!((angle_degrees - 45.0).abs() < 1e-6);
                assert_eq!(*duration_ms, 5);
            }
            other => panic!("unexpected instruction: {other:?}"),
        }
    }

    #[test]
    fn parses_case_insensitive_and_inline_comments() {
        let lines = ["ROTATE residue:7 angle: -30 deg duration: 2 ms # comment"];
        let contract = FoldingContract::from_lines(&lines);
        assert_eq!(contract.instructions.len(), 1);
        match &contract.instructions[0] {
            ContractInstruction::Rotate {
                residue,
                angle_degrees,
                duration_ms,
            } => {
                assert_eq!(residue.0, 7);
                assert!((*angle_degrees + 30.0).abs() < 1e-6);
                assert_eq!(*duration_ms, 2);
            }
            other => panic!("unexpected instruction: {other:?}"),
        }
    }

    #[test]
    fn parses_domain_chaperone_and_modification_directives() {
        let lines = [
            "define_domain helixA 5-20",
            "require_chaperone Hsp70 for helixA",
            "add_modification phosphorylation at S50",
            "set_physics_level GB",
            "physics_span on",
        ];
        let contract = FoldingContract::from_lines(&lines);
        assert_eq!(contract.instructions.len(), 5);

        match &contract.instructions[0] {
            ContractInstruction::DefineDomain { name, start, end } => {
                assert_eq!(name.as_deref(), Some("helixA"));
                assert_eq!(start.0, 5);
                assert_eq!(end.0, 20);
            }
            other => panic!("unexpected instruction: {other:?}"),
        }

        match &contract.instructions[1] {
            ContractInstruction::RequireChaperone { chaperone, span } => {
                assert_eq!(chaperone, "Hsp70");
                assert_eq!(span.as_deref(), Some("helixA"));
            }
            other => panic!("unexpected instruction: {other:?}"),
        }

        match &contract.instructions[2] {
            ContractInstruction::AddModification {
                modification,
                residue,
            } => {
                assert_eq!(modification, "phosphorylation");
                assert_eq!(residue.0, 50);
            }
            other => panic!("unexpected instruction: {other:?}"),
        }

        match &contract.instructions[3] {
            ContractInstruction::SetPhysicsLevel(level) => {
                assert_eq!(*level, PhysicsLevel::Gb);
            }
            other => panic!("unexpected instruction: {other:?}"),
        }

        match &contract.instructions[4] {
            ContractInstruction::SetSpanPhysics(mode) => {
                assert_eq!(*mode, PhysicsSpanMode::Physics);
            }
            other => panic!("unexpected instruction: {other:?}"),
        }
    }
}
