use anyhow::{anyhow, Result};
use log::{debug, info};
use yash_syntax::syntax::{
    self, CompoundCommand, RedirBody, RedirOp, SimpleCommand, TextUnit, WordUnit,
};

pub fn parse(script: &str) -> Result<syntax::List> {
    script
        .parse()
        .map_err(|err| anyhow!("Failed to parse shell script: {err:#}"))
}

pub fn validate(script: &str) -> Result<Vec<String>> {
    let parsed = parse(script)?;
    let mut findings = vec![];
    validate_ast(&parsed, &mut findings, &[])?;
    Ok(findings)
}

fn word_contains_variables(word: &syntax::Word) -> bool {
    word.units.iter().any(|unit| match unit {
        WordUnit::Unquoted(unit) => text_unit_contains_variables(unit),
        WordUnit::SingleQuote(_) => false,
        WordUnit::DoubleQuote(text) => text.0.iter().any(text_unit_contains_variables),
        WordUnit::Tilde(_) => false,
    })
}

fn text_unit_contains_variables(unit: &TextUnit) -> bool {
    match unit {
        TextUnit::Literal(_) => false,
        TextUnit::Backslashed(_) => false,
        TextUnit::RawParam { .. } => true,
        TextUnit::BracedParam(_) => true,
        TextUnit::CommandSubst { .. } => true,
        TextUnit::Backquote { .. } => false,
        TextUnit::Arith { content, .. } => content.0.iter().any(text_unit_contains_variables),
    }
}

fn get_subshell_command_subst(command: &SimpleCommand) -> Option<&str> {
    if !command.assigns.is_empty() {
        return None;
    }

    // ensure it's a single word
    let mut words = command.words.iter();
    let word = words.next()?;
    if words.next().is_some() {
        return None;
    }

    // ensure it contains a single unit
    let mut units = word.units.iter();
    let unit = units.next()?;
    if units.next().is_some() {
        return None;
    }

    match unit {
        WordUnit::Unquoted(TextUnit::CommandSubst { content, .. }) => Some(content),
        WordUnit::DoubleQuote(text) => {
            // ensure it contains a single text unit
            let mut text = text.0.iter();
            let first = text.next()?;
            if text.next().is_some() {
                return None;
            }

            match first {
                TextUnit::CommandSubst { content, .. } => Some(content),
                _ => None,
            }
        }
        _ => None,
    }
}

fn validate_simple_command(
    simple: &SimpleCommand,
    findings: &mut Vec<String>,
    function_stack: &[String],
) -> Result<()> {
    info!("Entering simple command processor");

    for assign in &simple.assigns {
        let name = assign.name.to_string();
        let value = assign.value.to_string();
        debug!("assign: {name:?}={value:?}");
    }

    // TODO: CommandSubst is not picked up if part of an arithmetic expression
    if let Some(script) = get_subshell_command_subst(simple) {
        debug!("Detected subshell command subst");
        let parsed = parse(script)?;
        validate_ast(&parsed, findings, function_stack)?;
    } else {
        let mut words = simple.words.iter();
        if let Some(first) = words.next() {
            let cmd = first.to_string();
            debug!("cmd={cmd:?}");

            if function_stack.is_empty() {
                findings.push(format!(
                    "Function call outside of any function: {:?}",
                    simple.to_string()
                ));
            }

            if word_contains_variables(first) {
                findings.push(format!("Command name contains variable: {cmd:?}"));
            } else {
                match cmd.as_str() {
                    "/bin/true" => (),
                    ":" => (),
                    "[" => (),
                    "break" => (),
                    "cat" => (),
                    "cd" => (),
                    "chmod" => (),
                    "chown" => (),
                    "continue" => (),
                    "echo" => (),
                    "getent" => (),
                    "grep" => (),
                    "mkdir" => (),
                    "pgrep" => (),
                    "post_install" => (),
                    "post_remove" => (),
                    "post_upgrade" => (),
                    "printf" => (),
                    "return" => (),
                    "rm" => (),
                    "rmdir" => (),
                    "setcap" => (),
                    "shift" => (),
                    "systemd-sysusers" => (),
                    "touch" => (),
                    "true" => (),
                    "usermod" => (),
                    "vercmp" => (),
                    _ => {
                        findings.push(format!(
                            "Running unrecognized command: {:?}",
                            simple.to_string()
                        ));
                    }
                }
            }

            for arg in words {
                let arg = arg.to_string();
                debug!("arg={arg:?}");
            }
        }
    }

    for redir in &*simple.redirs {
        validate_redir(redir, findings)?;
    }

    info!("Exiting simple command processor");

    Ok(())
}

fn validate_compound_command(
    compound: &CompoundCommand,
    findings: &mut Vec<String>,
    function_stack: &[String],
) -> Result<()> {
    match compound {
        CompoundCommand::Grouping(list) => {
            validate_ast(list, findings, function_stack)?;
        }
        CompoundCommand::Subshell { body, .. } => {
            validate_ast(body, findings, function_stack)?;
        }
        CompoundCommand::For { body, .. } => {
            validate_ast(body, findings, function_stack)?;
        }
        CompoundCommand::While {
            condition, body, ..
        } => {
            validate_ast(condition, findings, function_stack)?;
            validate_ast(body, findings, function_stack)?;
        }
        CompoundCommand::Until {
            condition, body, ..
        } => {
            validate_ast(condition, findings, function_stack)?;
            validate_ast(body, findings, function_stack)?;
        }
        CompoundCommand::If {
            condition,
            body,
            elifs,
            r#else,
        } => {
            info!("Entering if-expression processor");
            validate_ast(condition, findings, function_stack)?;
            validate_ast(body, findings, function_stack)?;
            for elif in elifs {
                validate_ast(&elif.condition, findings, function_stack)?;
                validate_ast(&elif.body, findings, function_stack)?;
            }
            if let Some(or_else) = r#else {
                validate_ast(or_else, findings, function_stack)?;
            }
            info!("Exiting if-expression processor");
        }
        CompoundCommand::Case { items, .. } => {
            for item in items {
                validate_ast(&item.body, findings, function_stack)?;
            }
        }
    }

    Ok(())
}

fn validate_redir(redir: &syntax::Redir, findings: &mut Vec<String>) -> Result<()> {
    // TODO: for inputs we should check redir.fd is none
    match &redir.body {
        RedirBody::Normal { operator, operand } => match operator {
            RedirOp::FileIn => {
                if let Some(fd) = redir.fd {
                    findings.push(format!("File input on unusual descriptor: fd={fd:?}"));
                }
            }
            RedirOp::FileInOut => {
                findings.push(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::FileOut => {
                let file = operand.to_string();
                if file != "/dev/null" {
                    findings.push(format!("File write to: {:?}", operand.to_string()));
                }
            }
            RedirOp::FileAppend => {
                let file = operand.to_string();
                if file != "/dev/null" {
                    findings.push(format!("File write to: {:?}", operand.to_string()));
                }
            }
            RedirOp::FileClobber => {
                findings.push(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::FdIn => {
                findings.push(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::FdOut => {
                let fd = operand.to_string();
                if fd != "1" && fd != "2" && fd != "-" {
                    findings.push(format!(
                        "File descriptor redirect to unusual descriptor: {fd:?}"
                    ));
                }
            }
            RedirOp::Pipe => {
                findings.push(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::String => {
                findings.push(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
        },
        RedirBody::HereDoc(_) => (),
    }
    Ok(())
}

pub fn validate_ast(
    script: &syntax::List,
    findings: &mut Vec<String>,
    function_stack: &[String],
) -> Result<()> {
    for item in &script.0 {
        for cmd in &item.and_or.first.commands {
            match cmd.as_ref() {
                syntax::Command::Function(fun) => {
                    let name = fun.name.to_string();
                    info!("Discovered function: {name:?}");
                    let mut function_stack = function_stack.to_owned();
                    function_stack.push(name);
                    validate_compound_command(&fun.body.command, findings, &function_stack)?;
                }
                syntax::Command::Simple(simple) => {
                    validate_simple_command(simple, findings, function_stack)?;
                }
                syntax::Command::Compound(compound) => {
                    validate_compound_command(&compound.command, findings, function_stack)?;
                    for redir in &*compound.redirs {
                        validate_redir(redir, findings)?;
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::OnceLock;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Entry {
        Findings(Vec<String>),
        Error(String),
    }

    static CELL: OnceLock<BTreeMap<String, Entry>> = OnceLock::new();

    fn harness() -> &'static BTreeMap<String, Entry> {
        CELL.get_or_init(|| {
            let bytes = fs::read("test_harness.json").unwrap();
            serde_json::from_slice(&bytes).unwrap()
        })
    }

    include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
}
