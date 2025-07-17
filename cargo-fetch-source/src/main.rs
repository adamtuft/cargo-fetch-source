use fetch_source::{self as fetch, Artefact};

use fetch::Parse;

fn main() {
    let out_dir = std::path::PathBuf::from(".");
    match std::fs::read_to_string("Cargo.toml") {
        Ok(document) => {
            match fetch::Sources::try_parse_toml(&document) {
                Ok(sources) => {
                    for (name, source) in sources {
                        match source.fetch(&name, out_dir.canonicalize().unwrap()) {
                            Ok(Artefact::Tarball { items }) => {
                                println!("Extracted {} into:", source.upstream());
                                for (dir, files) in items {
                                    println!(" => {:#?} ({} items)", out_dir.join(dir), files.len());
                                }
                            },
                            Ok(Artefact::Repository(path)) => println!("Fetched repository into {path:?}"),
                            Err(e) => eprintln!("Failed to fetch '{name}': {e}"),
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse Cargo.toml: {e}");
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read Cargo.toml: {e}");
        }
    }
}
