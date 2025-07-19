use anyhow::Context;

use fetch_source::{Source, Artefact};

pub fn fetch_serial(
    mut artefacts: Vec<Artefact>,
    name: String,
    source: Source,
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

pub fn fetch_parallel(
    mut handles: Vec<std::thread::JoinHandle<Result<Artefact, fetch_source::Error>>>,
    name: String,
    source: Source,
    out_dir: &std::path::Path,
) -> Vec<std::thread::JoinHandle<Result<Artefact, fetch_source::Error>>> {
    let out_dir = out_dir.to_path_buf();
    handles.push(std::thread::spawn(move || {
        source.fetch(&name, &out_dir)
    }));
    handles
}
