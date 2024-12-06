use crate::command;
use crate::errors::*;
use crate::redirect;
use yash_syntax::syntax::{self, CompoundCommand};

pub fn parse(script: &str) -> Result<syntax::List> {
    script
        .parse()
        .map_err(|err| anyhow!("Failed to parse shell script: {err:#}"))
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
                    command::validate_simple_command(simple, findings, function_stack)?;
                }
                syntax::Command::Compound(compound) => {
                    validate_compound_command(&compound.command, findings, function_stack)?;
                    for redir in &*compound.redirs {
                        redirect::validate_redir(redir, findings)?;
                    }
                }
            }
        }
    }

    Ok(())
}
