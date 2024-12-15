use clap::{ArgAction, Parser};
use colored::Colorize;
use env_logger::Env;
use smelly_hooks::command;
use smelly_hooks::errors::*;
use smelly_hooks::Context;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;

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
    /// Read the input file as .pkg.tar.zst, and write the install hook to this path,
    /// if one exists
    #[arg(long, value_name = "OUTPUT")]
    pub extract_from_pkg_to: Option<PathBuf>,
}

fn audit_script(args: &Args, script: &str) -> Result<()> {
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
        Ok(())
    } else {
        for finding in &findings {
            println!(
                "{}{}{} {}",
                "[".bold(),
                "!".bold().red(),
                "]".bold(),
                finding
            );
        }
        if findings.is_empty() {
            Ok(())
        } else {
            process::exit(2);
        }
    }
}

fn extract_hook<R: Read>(input: R, output: &Path) -> Result<()> {
    let decoder = ruzstd::StreamingDecoder::new(input)?;
    let mut tar = tar::Archive::new(decoder);

    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path == Path::new(".INSTALL") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;

            let mut output: Box<dyn Write> = if output == Path::new("-") {
                Box::new(io::stdout())
            } else {
                let file = File::create(&output)
                    .with_context(|| anyhow!("Failed to open output path: {output:?}"))?;
                Box::new(file)
            };

            output
                .write_all(&buf)
                .context("Failed to write to output")?;
            break;
        }
    }

    Ok(())
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

    let input: Box<dyn Read> = if args.path == Path::new("-") {
        Box::new(io::stdin())
    } else {
        let file = File::open(&args.path)
            .with_context(|| anyhow!("Failed to open input file: {:?}", args.path))?;
        Box::new(file)
    };

    if let Some(output) = args.extract_from_pkg_to {
        extract_hook(input, &output)
    } else {
        let script = io::read_to_string(input).context("Failed to read hook")?;
        audit_script(&args, &script)
    }
}
