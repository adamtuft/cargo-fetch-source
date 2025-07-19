use anyhow::Context;

use fetch::Parse;
use fetch_source::{self as fetch, Artefact};

mod args;
mod progress;

fn main() -> Result<(), anyhow::Error> {
    let args = args::parse()?;

    println!("{args:#?}");

    let document = std::fs::read_to_string(&args.manifest_file).context(format!(
        "Failed to read manifest file: {}",
        args.manifest_file.display()
    ))?;

    let artefacts = fetch::Sources::try_parse_toml(&document)
        .context("Failed to parse Cargo.toml")?
        .into_iter()
        .try_fold(
            Vec::new(),
            |mut artefacts, (name, source)| {
                let source_num = artefacts.len() + 1;
                println!("🔄 [{source_num}] Fetching source '{name}'...");
                match source.fetch(&name, &args.out_dir) {
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
                                        args.out_dir.join(dir).display(),
                                        files.len()
                                    );
                                }
                            }
                        }
                        artefacts.push(artefact);
                        Ok(artefacts)
                    }
                    Err(e) => Err(e).context(format!("Failed to fetch source '{name}'")),
                }
            },
        )?;

    println!("\n🎉 Successfully fetched {} source(s)!", artefacts.len());
    Ok(())
}
