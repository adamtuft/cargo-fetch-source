use anyhow::Context;

use fetch_source::{Source, Sources, Artefact};

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

pub fn fetch_in_parallel_scope(sources: Sources, out_dir: &std::path::Path) -> std::vec::Vec<std::result::Result<std::result::Result<fetch_source::Artefact, anyhow::Error>, std::boxed::Box<dyn std::any::Any + std::marker::Send>>> {
    use std::thread::{scope, ScopedJoinHandle};
    scope(move |scope| {
        let mut handles: Vec<ScopedJoinHandle<'_, Result<Artefact, anyhow::Error>>> = Vec::new();
        for (name, source) in sources {
            let h = scope.spawn(move || {
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
                Ok(artefact)
            });
            handles.push(h);
        }
        // Collect the results, then return them.
        handles
            .into_iter()
            .map(|h| h.join())
            .collect::<Vec<Result<Result<Artefact, anyhow::Error>, _>>>()
    }) // Implicitly return the results
}