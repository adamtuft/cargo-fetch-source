use anyhow::Context;

use fetch_source::{Source, Artefact};

pub fn fetch_serial(
    mut artefacts: Vec<Artefact>,
    (name, source): (String, Source),
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
