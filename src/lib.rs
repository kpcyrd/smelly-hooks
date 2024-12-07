pub mod ast;
pub mod command;
pub mod errors;
pub mod redirect;

use crate::errors::*;
use std::collections::BTreeSet;
use std::mem;

#[derive(Debug, PartialEq)]
pub enum FindingCondition {
    FunctionUndeclared(String),
}

impl FindingCondition {
    pub fn holds(&self, ctx: &Context) -> bool {
        match self {
            FindingCondition::FunctionUndeclared(name) => !ctx.declared_functions.contains(name),
        }
    }
}

pub struct Context {
    findings: Vec<(String, Vec<FindingCondition>)>,
    pub trusted_commands: BTreeSet<&'static str>,
    pub declared_functions: BTreeSet<String>,
    pub inside_compound: usize,
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
            findings: vec![],
            trusted_commands: BTreeSet::new(),
            declared_functions: BTreeSet::new(),
            inside_compound: 0,
        }
    }

    pub fn is_inside_compound(&self) -> bool {
        self.inside_compound != 0
    }

    pub fn finding(&mut self, finding: String) {
        self.finding_conditional(finding, vec![])
    }

    pub fn finding_conditional(&mut self, finding: String, conditions: Vec<FindingCondition>) {
        self.findings.push((finding, conditions));
    }

    pub fn validate(mut self, script: &str) -> Result<Vec<String>> {
        let parsed = ast::parse(script)?;
        ast::validate_ast(&mut self, &parsed, &[])?;
        let findings = mem::take(&mut self.findings)
            .into_iter()
            .filter(|(_, conditions)| conditions.iter().all(|c| c.holds(&self)))
            .map(|(finding, _)| finding)
            .collect();
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

    mod archlinux {
        use super::*;
        include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
    }
}
