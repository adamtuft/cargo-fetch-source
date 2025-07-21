use std::any::Any;

use anyhow::Context;

use fetch_source::{Artefact, Source, Sources};

pub type FetchResult = Result<Artefact, anyhow::Error>;

pub fn fetch_one_print_outcome<S: AsRef<str>>(
    name: S,
    source: Source,
    out_dir: &std::path::Path,
) -> FetchResult {
    println!("🔄 Fetching {}...", name.as_ref());
    let artefact = source
        .fetch(name.as_ref(), out_dir)
        .context(format!("Failed to fetch source '{}'", name.as_ref()))?;
    match artefact {
        Artefact::Git(ref repo) => {
            println!("✅ 🔗 Cloned repository into {}", repo.local.display());
        }
        Artefact::Tar(ref tar) => {
            println!("✅ 📦 Extracted {} into {}", tar.url, out_dir.display());
        }
    }
    Ok(artefact)
}

pub fn fetch_one_progress_outcome<S: AsRef<str>>(
    name: S,
    source: Source,
    out_dir: &std::path::Path,
    bar: indicatif::ProgressBar,
) -> FetchResult {
    bar.set_message(format!("🔄 Fetching {}...", name.as_ref()));
    let result = source.fetch(name.as_ref(), out_dir);
    let status = match result {
        Ok(Artefact::Git(ref repo)) => {
            format!("✅ 🔗 Cloned repository into {}", repo.local.display())
        }
        Ok(Artefact::Tar(ref tar)) => {
            format!("✅ 📦 Extracted {} into {}", tar.url, out_dir.display())
        }
        Err(ref e) => format!("❌ Failed to fetch source: {e}"),
    };
    bar.finish_with_message(status);
    Ok(result?)
}

pub fn fetch_serial(
    mut artefacts: Vec<Artefact>,
    name: String,
    source: Source,
    out_dir: &std::path::Path,
) -> Result<Vec<Artefact>, anyhow::Error> {
    let artefact = fetch_one_print_outcome(name, source, out_dir)?;
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
    handles.push(std::thread::spawn(move || source.fetch(&name, &out_dir)));
    handles
}

pub fn fetch_in_parallel_scope(
    sources: Sources,
    out_dir: &std::path::Path,
) -> Vec<Result<FetchResult, Box<dyn Any + Send>>> {
    std::thread::scope(move |scope| {
        sources
            .into_iter()
            .map(|(n, s)| scope.spawn(move || fetch_one_print_outcome(n, s, out_dir)))
            .map(|h| h.join())
            .collect::<Vec<Result<FetchResult, _>>>()
    })
}

pub fn fetch_in_parallel_scope_with_multiprogress(
    sources: Sources,
    out_dir: &std::path::Path,
) -> Vec<Result<FetchResult, Box<dyn Any + Send>>> {
    let mp = indicatif::MultiProgress::new();
    let bars: Vec<_> = (0..sources.len())
        .map(|_| crate::progress::make_progress_spinner(&mp))
        .collect();
    std::thread::scope(move |scope| {
        let handles = sources
            .into_iter()
            .zip(bars)
            .map(|((n, s), b)| scope.spawn(move || fetch_one_progress_outcome(n, s, out_dir, b)))
            .collect::<Vec<_>>();
        handles.into_iter().map(|h| h.join()).collect()
    })
}
