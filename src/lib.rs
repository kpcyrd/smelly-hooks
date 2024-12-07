pub mod ast;
pub mod command;
pub mod errors;
pub mod redirect;

use crate::errors::*;
use std::collections::BTreeSet;

pub struct Context {
    pub trusted_commands: BTreeSet<&'static str>,
}

impl Default for Context {
    fn default() -> Self {
        let mut ctx = Self::empty();
        ctx.trusted_commands.extend(command::REASONABLE_BINARIES);
        ctx.trusted_commands.extend(command::REASONABLE_BUILTINS);
        ctx
    }
}

impl Context {
    pub fn empty() -> Self {
        Context {
            trusted_commands: BTreeSet::new(),
        }
    }

    pub fn validate(&self, script: &str) -> Result<Vec<String>> {
        let parsed = ast::parse(script)?;
        let mut findings = vec![];
        ast::validate_ast(self, &parsed, &mut findings, &[])?;
        Ok(findings)
    }
}

pub fn validate(script: &str) -> Result<Vec<String>> {
    Context::default().validate(script)
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
