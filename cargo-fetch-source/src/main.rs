use anyhow::Context;

use fetch::Parse;
use fetch_source::{self as fetch, Artefact};

mod args;
mod progress;

fn fetch_source(
    mut artefacts: Vec<Artefact>,
    (name, source): (String, fetch::Source),
    out_dir: &std::path::Path,
) -> Result<Vec<Artefact>, anyhow::Error> {
    println!("🔄 Fetching {name}...");
    let artefact = source.fetch(&name, out_dir)
        .context(format!("Failed to fetch source '{name}'"))?;
    match artefact {
        Artefact::Git(ref repo) => {
            println!("✅ 🔗 Cloned repository into {}", repo.local.display());
        }
        Artefact::Tar(ref tar) => {
            println!("✅ 📦 Extracted {} into {}", tar.url, out_dir.display());
        }
    }
    artefacts.push(artefact);
    Ok(artefacts)
}

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
        .try_fold(Vec::new(), |artefacts, element| {
            fetch_source(artefacts, element, &args.out_dir)
        })?;

    println!("\n🎉 Successfully fetched {} source(s)!", artefacts.len());
    Ok(())
}
