use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::Parser;

use fetch::Parse;
use fetch_source::{self as fetch, Artefact};

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
    /// directories
    #[arg(long, short = 'm', value_name = "PATH")]
    manifest_file: Option<PathBuf>,

    /// Output directory for fetched sources. If absent, try the `OUT_DIR` environment variable,
    /// then fall back to the current working directory. The given directory must exist.
    #[arg(long, short = 'o', value_name = "PATH")]
    out_dir: Option<PathBuf>,
}

fn main() -> Result<(), anyhow::Error> {
    let mut args = Args::parse();

    // If the manifest path is not provided, search for it in the directory hierarchy.
    if args.manifest_file.is_none() {
        let mut current_dir = std::env::current_dir()?;
        loop {
            let manifest = current_dir.join("Cargo.toml");
            if manifest.is_file() {
                args.manifest_file = Some(manifest);
                break;
            }
            if !current_dir.pop() {
                bail!(
                    "could not find `Cargo.toml` in the current directory or any parent directory"
                );
            }
        }
    }

    // If the output directory is not provided, try to use `OUT_DIR` and fall bacl to the current directory.
    if args.out_dir.is_none() {
        args.out_dir = match std::env::var("OUT_DIR") {
            Ok(s) => Some(std::path::PathBuf::from(s)),
            Err(_) => Some(std::env::current_dir()?),
        };
    }

    println!("{args:#?}");

    // SAFETY: we have just ensured these values are Some(_)
    let out_dir = args.out_dir.unwrap().canonicalize()?;
    let manifest_file = args.manifest_file.unwrap();
    let document = std::fs::read_to_string(&manifest_file).context(format!(
        "Failed to read manifest file: {}",
        manifest_file.display()
    ))?;

    let context = "hello!".to_string();
    let callback = |name: &str,
                    source: fetch::Source,
                    out_dir: &Path,
                    ctx: &String|
     -> Result<(), anyhow::Error> {
        match source.fetch(name, out_dir) {
            Ok(Artefact::Tar(tar)) => {
                println!("Extracted {} into:", tar.url);
                for (dir, files) in tar.items {
                    println!(" => {:#?} ({} items)", out_dir.join(dir), files.len());
                }
            }
            Ok(Artefact::Git(path)) => {
                println!("Fetched repository into {path:?}")
            }
            Err(e) => {
                return Err(e).context(format!("Failed to fetch source '{name}'"));
            }
        }
        Ok(())
    };

    process_sources_with_callback(&document, &out_dir, &context, callback)
}

fn process_sources_with_callback<T, F>(
    document: &str,
    out_dir: &Path,
    context: &T,
    callback: F,
) -> Result<(), anyhow::Error>
where
    F: Fn(&str, fetch::Source, &Path, &T) -> Result<(), anyhow::Error>,
{
    let sources = fetch::Sources::try_parse_toml(document).context("Failed to parse Cargo.toml")?;
    for (name, source) in sources {
        callback(&name, source, out_dir, context)?;
    }
    Ok(())
}
