use std::path::PathBuf;

use anyhow::{anyhow, bail};
use clap::Parser;

use crate::error::AppError;

// Shamelessly borrowed from https://github.com/crate-ci/clap-cargo/blob/0378657ffdf2b67bcd6f1ab56e04a1322b92dd0e/src/style.rs
// thanks to https://stackoverflow.com/a/79614957
mod style {
    #![allow(dead_code)]
    use anstyle::AnsiColor;
    use anstyle::Effects;
    use anstyle::Style;

    pub const NOP: Style = Style::new();
    pub const HEADER: Style = AnsiColor::Green.on_default().effects(Effects::BOLD);
    pub const USAGE: Style = AnsiColor::Green.on_default().effects(Effects::BOLD);
    pub const LITERAL: Style = AnsiColor::Cyan.on_default().effects(Effects::BOLD);
    pub const PLACEHOLDER: Style = AnsiColor::Cyan.on_default();
    pub const ERROR: Style = AnsiColor::Red.on_default().effects(Effects::BOLD);
    pub const WARN: Style = AnsiColor::Yellow.on_default().effects(Effects::BOLD);
    pub const NOTE: Style = AnsiColor::Cyan.on_default().effects(Effects::BOLD);
    pub const GOOD: Style = AnsiColor::Green.on_default().effects(Effects::BOLD);
    pub const VALID: Style = AnsiColor::Cyan.on_default().effects(Effects::BOLD);
    pub const INVALID: Style = AnsiColor::Yellow.on_default().effects(Effects::BOLD);

    pub const CLAP_STYLING: clap::builder::styling::Styles =
        clap::builder::styling::Styles::styled()
            .header(HEADER)
            .usage(USAGE)
            .literal(LITERAL)
            .placeholder(PLACEHOLDER)
            .error(ERROR)
            .valid(VALID)
            .invalid(INVALID);
}

#[derive(Debug, Parser)]
#[command(name = "cargo-fetch-source")]
#[command(about = "Fetch external source trees specified in Cargo.toml")]
#[command(version, long_about = None)]
#[command(styles = style::CLAP_STYLING)]
#[command(term_width = 80)]
struct Args {
    /// Path to the Cargo.toml file. If not given, search for the file in the current and parent
    /// directories.
    #[arg(long, short = 'm', value_name = "PATH")]
    manifest_file: Option<PathBuf>,

    /// Output directory for fetched sources. If absent, try the `OUT_DIR` environment variable,
    /// then fall back to the current working directory. The given directory must exist.
    #[arg(long, short = 'o', value_name = "PATH")]
    out_dir: Option<PathBuf>,

    /// Number of threads to spawn. Defaults to one per logical CPU.
    #[arg(long, short = 't', value_name = "NUM-THREADS")]
    threads: Option<u32>,
}

#[derive(Debug)]
pub struct ValidatedArgs {
    pub manifest_file: PathBuf,
    pub out_dir: PathBuf,
    pub threads: Option<u32>,
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
                    return Err(AppError::ArgValidation(format!(
                        "could not find `Cargo.toml` in the current directory or any parent directory"
                    )));
                }
            }
        };

        // If the output directory is not provided, try to use `OUT_DIR` and fall back to the current directory.
        let out_dir = match args.out_dir {
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

        Ok(ValidatedArgs {
            manifest_file,
            out_dir: out_dir.canonicalize()?,
            threads: args.threads,
        })
    }
}

pub fn parse() -> Result<ValidatedArgs, AppError> {
    ValidatedArgs::try_from(Args::parse())
}
