use std::path::PathBuf;

use anyhow::Context;
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
    #[arg(long, short = 'm')]
    manifest_file: Option<PathBuf>,

    /// Output directory for fetched sources. If absent, try the `OUT_DIR` environment variable,
    /// then fall back to the current working directory. The given directory must exist.
    #[arg(long, short = 'o')]
    out_dir: Option<PathBuf>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    println!("{args:#?}");

    match std::fs::read_to_string(&args.manifest_file) {
        Ok(document) => match fetch::Sources::try_parse_toml(&document) {
            Ok(sources) => {
                for (name, source) in sources {
                    match source.fetch(&name, args.out_dir.canonicalize().unwrap()) {
                        Ok(Artefact::Tar(tar)) => {
                            println!("Extracted {} into:", tar.url);
                            for (dir, files) in tar.items {
                                println!(
                                    " => {:#?} ({} items)",
                                    args.out_dir.join(dir),
                                    files.len()
                                );
                            }
                        }
                        Ok(Artefact::Git(path)) => {
                            println!("Fetched repository into {path:?}")
                        }
                        Err(e) => {
                            return Err(e)
                                .context(format!("Failed to fetch source '{name}'"));
                        }
                    }
                }
            }
            Err(e) => {
                return Err(e).context("Failed to parse Cargo.toml");
            }
        },
        Err(e) => {
            return Err(e).context("Failed to read Cargo.toml");
        }
    }
    Ok(())
}
