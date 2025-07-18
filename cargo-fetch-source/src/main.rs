use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

use fetch::Parse;
use fetch_source::{self as fetch, Artefact};

#[derive(Parser)]
#[command(name = "cargo-fetch-source")]
#[command(about = "Fetch external sources specified in Cargo.toml")]
struct Args {
    /// Path to the Cargo.toml file
    #[arg(long, short = 'm', default_value = "Cargo.toml")]
    manifest_file: PathBuf,

    /// Output directory for fetched sources
    #[arg(long, short = 'o', default_value = ".")]
    out_dir: PathBuf,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

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
