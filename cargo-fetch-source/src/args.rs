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
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Fetch the sources specified in the manifest
    Fetch {
        /// Path to the Cargo.toml file. If not given, search for the file in the current and parent
        /// directories.
        #[arg(long, short = 'm', value_name = "PATH", global = true)]
        manifest_file: Option<PathBuf>,

        /// Output directory for fetched sources. If absent, try the `OUT_DIR` environment variable,
        /// then fall back to the current working directory. The given directory must exist.
        #[arg(long, short = 'o', value_name = "PATH")]
        out_dir: Option<PathBuf>,

        /// Cache directory to use.
        #[arg(long = "cache", short = 'c', value_name = "PATH")]
        cache_dir: Option<PathBuf>,

        /// Number of threads to spawn. Defaults to one per logical CPU.
        #[arg(long, short = 't', value_name = "NUM-THREADS")]
        threads: Option<u32>,
    },
    /// List the sources specified in the manifest without fetching them
    List {
        /// Path to the Cargo.toml file. If not given, search for the file in the current and parent
        /// directories.
        #[arg(long, short = 'm', value_name = "PATH", global = true)]
        manifest_file: Option<PathBuf>,
        /// Output format
        #[arg(long, short = 'f', value_enum, value_name = "FORMAT")]
        format: Option<OutputFormat>,
    },
    /// List the cached sources. Defaults to `CARGO_FETCH_SOURCE_CACHE` environment variable
    /// then `~/.cache/cargo-fetch-source`
    Cached {
        /// Output format
        #[arg(long, short = 'f', value_enum, value_name = "FORMAT")]
        format: Option<OutputFormat>,
        /// Cache directory to list.
        #[arg(long = "cache", short = 'c', value_name = "PATH")]
        cache_dir: Option<PathBuf>,
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
    pub command: ValidatedCommand,
}

#[derive(Debug)]
pub enum ValidatedCommand {
    Fetch {
        manifest_file: PathBuf,
        out_dir: PathBuf,
        cache: fetch_source::Cache,
    },
    List {
        manifest_file: PathBuf,
        format: Option<OutputFormat>,
    },
    Cached {
        format: Option<OutputFormat>,
        cache: fetch_source::Cache,
    },
}

impl ValidatedArgs {
    fn detect_out_dir(arg: Option<PathBuf>) -> Result<PathBuf, AppError> {
        Ok(match arg {
            Some(path) => path,
            None => match std::env::var("OUT_DIR") {
                Ok(s) => std::path::PathBuf::from(s),
                Err(_) => std::env::current_dir()?,
            },
        })
    }

    fn detect_manifest_file(arg: Option<PathBuf>) -> Result<PathBuf, AppError> {
        match arg {
            Some(path) => Ok(path),
            None => {
                let mut current_dir = std::env::current_dir()?;
                loop {
                    let manifest = current_dir.join("Cargo.toml");
                    if manifest.is_file() {
                        break Ok(manifest);
                    }
                    if !current_dir.pop() {
                        return Err(AppError::arg_validation(
                            "could not find 'Cargo.toml' in the current directory or any parent directory".to_string(),
                        ));
                    }
                }
            }
        }
    }

    fn detect_cache_dir(arg: Option<PathBuf>) -> Result<PathBuf, AppError> {
        match arg {
            Some(dir) => Ok(dir),
            None => match std::env::var_os("CARGO_FETCH_SOURCE_CACHE") {
                Some(dir) => Ok(PathBuf::from(dir)),
                None => {
                    let project_dirs = directories::ProjectDirs::from("", "", "cargo-fetch-source")
                        .ok_or(AppError::arg_validation(
                            "could not determine cache directory".to_string(),
                        ))?;
                    Ok(project_dirs.cache_dir().to_path_buf())
                }
            },
        }
    }

    /// Loads the cache from the given directory, creating a new cache if the file does not exist.
    /// Also creates the directory if it does not exist.
    fn load_cache_from(cache_dir: std::path::PathBuf) -> Result<fetch_source::Cache, AppError> {
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir)?;
        }
        fetch_source::Cache::load_or_create(&cache_dir).map_err(|e| {
            AppError::arg_validation(format!(
                "failed to load cache in {}: {}",
                cache_dir.display(),
                e
            ))
        })
    }
}

impl TryFrom<Command> for ValidatedCommand {
    type Error = AppError;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Fetch {
                manifest_file,
                out_dir,
                cache_dir,
                threads,
            } => {
                // If the output directory is not provided, try to use `OUT_DIR` and fall back to the current directory.
                let out_dir = ValidatedArgs::detect_out_dir(out_dir)?;

                // Validate that the output directory exists
                if !out_dir.exists() {
                    return Err(AppError::arg_validation(format!(
                        "output directory does not exist: {}",
                        out_dir.display()
                    )));
                }

                let cache_dir = ValidatedArgs::detect_cache_dir(cache_dir)?;
                let cache = ValidatedArgs::load_cache_from(cache_dir)?;

                if let Some(threads) = threads {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(threads as usize)
                        .build_global()
                        .map_err(|e| {
                            AppError::arg_validation(format!("Failed to set thread count: {e}"))
                        })?;
                }

                Ok(ValidatedCommand::Fetch {
                    manifest_file: ValidatedArgs::detect_manifest_file(manifest_file)?,
                    out_dir: out_dir.canonicalize()?,
                    cache,
                })
            }
            Command::List {
                manifest_file,
                format,
            } => Ok(ValidatedCommand::List {
                manifest_file: ValidatedArgs::detect_manifest_file(manifest_file)?,
                format,
            }),
            Command::Cached {
                format,
                cache_dir: cache_dir_arg,
            } => {
                let cache_dir = ValidatedArgs::detect_cache_dir(cache_dir_arg)?;
                // For the cached command, don't create the cache directory if it doesn't exist
                let cache = fetch_source::Cache::load(&cache_dir).map_err(|e| {
                    AppError::arg_validation(format!(
                        "failed to load cache in {}: {}",
                        cache_dir.display(),
                        e
                    ))
                })?;
                Ok(ValidatedCommand::Cached { format, cache })
            }
        }
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
    Ok(ValidatedArgs {
        command: ValidatedCommand::try_from(args.command)?,
    })
}
