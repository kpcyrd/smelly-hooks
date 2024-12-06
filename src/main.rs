use anyhow::{anyhow, Context as _, Result};
use clap::{ArgAction, Parser};
use colored::Colorize;
use env_logger::Env;
use smelly_hooks::validate;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(version)]
struct Args {
    /// Increase logging output (can be used multiple times)
    #[arg(short, long, global = true, action(ArgAction::Count))]
    pub verbose: u8,
    /// The install hook file to process
    pub path: PathBuf,
    #[arg(long)]
    pub json: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = match args.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    env_logger::init_from_env(Env::default().default_filter_or(log_level));

    let script = if args.path == Path::new("-") {
        io::read_to_string(io::stdin())?
    } else {
        fs::read_to_string(&args.path)
            .with_context(|| anyhow!("Failed to read hook from file: {:?}", args.path))?
    };

    let findings = validate(&script)?;
    if args.json {
        serde_json::to_writer(&io::stdout(), &findings)?;
        println!();
    } else {
        for finding in findings {
            println!(
                "{}{}{} {}",
                "[".bold(),
                "!".bold().red(),
                "]".bold(),
                finding
            );
        }
    }

    Ok(())
}
