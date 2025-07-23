#![allow(dead_code)]

use std::path::PathBuf;

use clap::Parser;

use crate::error::AppError;

// Shamelessly borrowed from https://github.com/crate-ci/clap-cargo/blob/0378657ffdf2b67bcd6f1ab56e04a1322b92dd0e/src/style.rs
// thanks to https://stackoverflow.com/a/79614957
use anstyle::AnsiColor::*;
use anstyle::Effects;
use anstyle::Style;

const NOP: Style = Style::new();
const HEADER: Style = Green.on_default().effects(Effects::BOLD);
const USAGE: Style = Green.on_default().effects(Effects::BOLD);
const LITERAL: Style = Cyan.on_default().effects(Effects::BOLD);
const PLACEHOLDER: Style = Cyan.on_default();
const ERROR: Style = Red.on_default().effects(Effects::BOLD);
const WARN: Style = Yellow.on_default().effects(Effects::BOLD);
const NOTE: Style = Cyan.on_default().effects(Effects::BOLD);
const GOOD: Style = Green.on_default().effects(Effects::BOLD);
const VALID: Style = Cyan.on_default().effects(Effects::BOLD);
const INVALID: Style = Yellow.on_default().effects(Effects::BOLD);

const APP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(HEADER)
    .usage(USAGE)
    .literal(LITERAL)
    .placeholder(PLACEHOLDER)
    .error(ERROR)
    .valid(VALID)
    .invalid(INVALID);

#[derive(Debug, Parser)]
#[command(name = "cargo-fetch-source")]
#[command(about = "Fetch external source trees specified in Cargo.toml")]
#[command(version, long_about = None)]
#[command(styles = APP_STYLING)]
#[command(term_width = 80)]
struct Args {
    /// Path to the Cargo.toml file. If not given, search for the file in the current and parent
    /// directories.
    #[arg(long, short = 'm', value_name = "PATH", global = true)]
    manifest_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Fetch the sources specified in the manifest
    Fetch {
        /// Output directory for fetched sources. If absent, try the `OUT_DIR` environment variable,
        /// then fall back to the current working directory. The given directory must exist.
        #[arg(long, short = 'o', value_name = "PATH")]
        out_dir: Option<PathBuf>,

        /// Number of threads to spawn. Defaults to one per logical CPU.
        #[arg(long, short = 't', value_name = "NUM-THREADS")]
        threads: Option<u32>,
    },
    /// List the sources specified in the manifest without fetching them
    List {
        /// Output format
        #[arg(long, short = 'f', value_enum, value_name = "FORMAT")]
        format: Option<OutputFormat>,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    /// Output in JSON format
    Json,
    /// Output in TOML format
    Toml,
}

#[derive(Debug)]
pub struct ValidatedArgs {
    pub manifest_file: PathBuf,
    pub command: ValidatedCommand,
}

#[derive(Debug)]
pub enum ValidatedCommand {
    Fetch {
        out_dir: PathBuf,
        threads: Option<u32>,
    },
    List {
        format: Option<OutputFormat>,
    },
}

impl TryFrom<Args> for ValidatedArgs {
    type Error = AppError;

    fn try_from(args: Args) -> Result<Self, Self::Error> {
        // If the manifest path is not provided, search for it in the directory hierarchy.
        let manifest_file = if let Some(path) = args.manifest_file {
            path
        } else {
            let mut current_dir = std::env::current_dir()?;
            loop {
                let manifest = current_dir.join("Cargo.toml");
                if manifest.is_file() {
                    break manifest;
                }
                if !current_dir.pop() {
                    return Err(AppError::ArgValidation("could not find 'Cargo.toml' in the current directory or any parent directory".to_string()));
                }
            }
        };

        let command = match args.command {
            Command::Fetch { out_dir, threads } => {
                // If the output directory is not provided, try to use `OUT_DIR` and fall back to the current directory.
                let out_dir = match out_dir {
                    Some(path) => path,
                    None => match std::env::var("OUT_DIR") {
                        Ok(s) => std::path::PathBuf::from(s),
                        Err(_) => std::env::current_dir()?,
                    },
                };

                // Validate that the output directory exists
                if !out_dir.exists() {
                    return Err(AppError::ArgValidation(format!(
                        "output directory does not exist: {}",
                        out_dir.display()
                    )));
                }

                ValidatedCommand::Fetch {
                    out_dir: out_dir.canonicalize()?,
                    threads,
                }
            }
            Command::List { format } => ValidatedCommand::List { format },
        };

        Ok(ValidatedArgs {
            manifest_file,
            command,
        })
    }
}

pub fn parse() -> Result<ValidatedArgs, AppError> {
    let raw_args = std::env::args().collect::<Vec<_>>();
    // If run via `cargo fetch-source` skip the command argument which cargo passes to the binary.
    let args = if raw_args.len() > 1 && raw_args[1] == "fetch-source" {
        Args::parse_from(&raw_args[1..])
    } else {
        Args::parse_from(&raw_args)
    };
    ValidatedArgs::try_from(args)
}
