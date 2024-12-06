use anyhow::{anyhow, bail, Result};
use log::{debug, info};
use yash_syntax::syntax::{self, TextUnit, WordUnit};

pub fn validate(script: &str) -> Result<Vec<String>> {
    let parsed = script
        .parse::<syntax::List>()
        .map_err(|err| anyhow!("Failed to parse shell script: {err:#}"))?;
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

pub fn validate_ast(
    script: &syntax::List,
    findings: &mut Vec<String>,
    function_stack: &[String],
) -> Result<()> {
    for item in &script.0 {
        for cmd in &item.and_or.first.commands {
            // println!("item={:?}", cmd);
            match cmd.as_ref() {
                syntax::Command::Function(fun) => {
                    let name = fun.name.to_string();
                    debug!("Discovered function: {name:?}");

                    /*
                    match &fun.body.command {
                        syntax::CompoundCommand::Grouping(list) => validate_ast(&list, &mut findings)?,
                        _ => todo!(),
                    }
                    */

                    // TODO: process function

                    /*
                    match name.as_str() {
                        "post_install" => (),
                        "post_upgrade" => (),
                        "pre_remove" => (),
                        "post_remove" => (),
                        "pre_upgrade" => (),
                        "pre_install" => (),
                        other => (), // todo!("Unknown function name: {other}"),
                    }
                    */
                }
                syntax::Command::Simple(simple) => {
                    info!("simple start");
                    for assign in &simple.assigns {
                        let name = assign.name.to_string();
                        let value = assign.value.to_string();
                        debug!("assign: {name:?}={value:?}");
                    }

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
                                "shift" => (),
                                _ => {
                                    findings.push(format!("Running unrecognized command: {cmd:?}"));
                                }
                            }
                        }

                        for arg in words {
                            let arg = arg.to_string();
                            debug!("arg={arg:?}");
                            /*
                            debug!("word={word:?}");
                            debug!("word={:?}", word.to_string());
                            */
                        }
                    }

                    for redir in &*simple.redirs {
                        debug!("redir={redir:?}");
                    }

                    info!("simple end");
                }
                syntax::Command::Compound(_) => {
                    bail!("Support for compound commands is not implemented yet")
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
