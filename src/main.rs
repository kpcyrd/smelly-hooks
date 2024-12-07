use clap::{ArgAction, Parser};
use colored::Colorize;
use env_logger::Env;
use smelly_hooks::command;
use smelly_hooks::errors::*;
use smelly_hooks::Context;
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
    /// Output json instead of human readable text
    #[arg(long)]
    pub json: bool,
    /// Reduce set of reasonable binaries (use twice to also remove builtins)
    #[arg(short = 'W', long, action(ArgAction::Count))]
    pub trust_fewer_commands: u8,
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

    // setup an empty configuration context
    let mut ctx = Context::empty();

    // register trusted commands
    if args.trust_fewer_commands < 1 {
        ctx.trusted_commands.extend(command::REASONABLE_BINARIES);
    }
    if args.trust_fewer_commands < 2 {
        ctx.trusted_commands.extend(command::REASONABLE_BUILTINS);
    }

    // run the configured audit
    let findings = ctx.validate(&script)?;
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
