use std::path::PathBuf;

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

#[derive(Debug)]
struct ValidatedArgs {
    manifest_file: PathBuf,
    out_dir: PathBuf,
}

impl TryFrom<Args> for ValidatedArgs {
    type Error = anyhow::Error;

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
                    bail!(
                        "could not find `Cargo.toml` in the current directory or any parent directory"
                    );
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
            bail!("Output directory does not exist: {}", out_dir.display());
        }

        Ok(ValidatedArgs {
            manifest_file,
            out_dir,
        })
    }
}

fn main() -> Result<(), anyhow::Error> {
    let args = ValidatedArgs::try_from(Args::parse())?;

    println!("{args:#?}");

    let out_dir = args.out_dir.canonicalize()?;
    let manifest_file = args.manifest_file;
    let document = std::fs::read_to_string(&manifest_file).context(format!(
        "Failed to read manifest file: {}",
        manifest_file.display()
    ))?;

    // Approach 1: Use try_fold to accumulate results and provide progress tracking
    let (total_sources, _artefacts) = fetch::Sources::try_parse_toml(&document)
        .context("Failed to parse Cargo.toml")?
        .into_iter()
        .try_fold(
            (0usize, Vec::new()),
            |(count, mut artefacts), pair| {
                let (name, source) = pair;
                let source_num = count + 1;
                println!("🔄 [{source_num}] Fetching source '{name}'...");
                match source.fetch(&name, &out_dir) {
                    Ok(artefact) => {
                        match artefact {
                            Artefact::Git(ref path) => {
                                println!("✅ 🔗 Cloned repository into {path:?}");
                            }
                            Artefact::Tar(ref tar) => {
                                println!("✅ 📦 Extracted {} into:", tar.url);
                                for (dir, files) in &tar.items {
                                    println!(
                                        "   └─ {:?} ({} items)",
                                        out_dir.join(dir).display(),
                                        files.len()
                                    );
                                }
                            }
                        }
                        artefacts.push(artefact);
                        Ok((source_num, artefacts))
                    }
                    Err(e) => Err(e).context(format!("Failed to fetch source '{name}'")),
                }
            },
        )?;

    println!("\n🎉 Successfully fetched {total_sources} source(s)!");
    Ok(())
}
