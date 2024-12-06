use anyhow::Result;
use clap::Parser;
use serde::Serialize;
use smelly_hooks::validate;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Args {
    pub path: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Entry {
    Findings(Vec<String>),
    Error(String),
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut harness = BTreeMap::<String, Entry>::new();
    for entry in fs::read_dir(&args.path)? {
        let entry = entry?.path();

        let Some(filename) = entry.file_name() else {
            continue;
        };
        let Some(filename) = filename.to_str() else {
            continue;
        };
        let Some(pkg) = filename.strip_suffix(".install") else {
            continue;
        };

        let script = fs::read_to_string(&entry)?;
        match validate(&script) {
            Ok(findings) if !findings.is_empty() => {
                harness.insert(pkg.to_string(), Entry::Findings(findings));
            }
            Ok(_) => (),
            Err(err) => {
                harness.insert(pkg.to_string(), Entry::Error(err.to_string()));
            }
        }
    }

    serde_json::to_writer_pretty(io::stdout(), &harness)?;
    println!();

    Ok(())
}
