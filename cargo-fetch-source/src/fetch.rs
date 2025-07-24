use fetch_source::{Artefact, Cache, CachedState, SourceArtefact, Source, Sources, CachedSources};

use CachedState::*;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub type FetchResult = Result<SourceArtefact, anyhow::Error>;

fn make_progress_spinner(m: &MultiProgress, prefix: String) -> ProgressBar {
    let pb = m.add(ProgressBar::new_spinner());
    // pb.set_style(ProgressStyle::default_spinner());
    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template(
            "{prefix:.cyan.bold/blue.bold} ⏳  {msg:.cyan/blue} {spinner}",
        )
        .unwrap()
        .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
    );
    pb.set_prefix(prefix);
    pb
}

fn complete_progress_bar(
    pb: ProgressBar,
    result: &Result<SourceArtefact, fetch_source::Error>,
    name: &str,
) {
    let template = if result.is_ok() {
        "{prefix:.cyan.bold/blue.bold} {msg:.cyan/blue}"
    } else {
        "{prefix:.cyan.bold/blue.bold} {msg:.red.bold}"
    };
    pb.set_style(ProgressStyle::with_template(template).unwrap());
    let status = if let Ok(fetched) = &result {
        let local = match fetched.artefact() {
            Artefact::Git(repo) => repo.local.display(),
            Artefact::Tar(tar) => tar.path.display(),
        };
        format!("✅  {name} -> {local}")
    } else {
        format!("⚠️  failed to fetch '{name}'")
    };
    pb.finish_with_message(status);
}

// Fetch a single source, reporting progress in the provided progress bar
fn fetch_one<S, P>(
    name: S,
    state: CachedState,
    source: Source,
    out_dir: P,
    bar: ProgressBar,
) -> Option<FetchResult>
where
    S: AsRef<str>,
    P: AsRef<std::path::Path>,
{
    match state {
        Cached => {
            bar.finish_with_message(format!("📦  {} -> <cached>", name.as_ref()));
            None
        }
        NotCached => {
            bar.set_message(format!("{} -> ", name.as_ref()));
            let result = source.fetch(name.as_ref(), &out_dir);
            complete_progress_bar(bar, &result, name.as_ref());
            Some(result.map_err(|e| e.into()))
        }
    }
}

// Fetch sources in parallel with `rayon`. Pair each source with its own progress bar. Using
// ordered bars means the bars are shown in order
pub fn parallel_fetch<P, S>(sources: CachedSources<S>, out_dir: P) -> Vec<FetchResult>
where
    P: AsRef<std::path::Path> + Sync,
    S: AsRef<str> + Sync + Send,
{
    use rayon::prelude::*;
    let count = sources.len();
    let mp = MultiProgress::new();
    let ordered_bars = (0..count)
        .map(|k| make_progress_spinner(&mp, format!("[{}/{count}]", k + 1)))
        .collect::<Vec<_>>();
    ordered_bars
        .into_iter()
        .zip(sources)
        // Have to `collect()` first because we can't use parallel iterator with Zip
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|(bar, (name, (state, source)))| {
            fetch_one(name, state, source, out_dir.as_ref(), bar)
        })
        .collect::<Vec<_>>()
        ;
    todo!()
}

// Same as `parallel_fetch`, but only fetches sources that are not cached.
// pub fn parallel_fetch_uncached<P>(sources: Sources, out_dir: P, cache: &Cache) -> Vec<FetchResult>
// where
//     P: AsRef<std::path::Path> + Sync,
// {
//     use rayon::prelude::*;
//     let count = sources.len();
//     let mp = MultiProgress::new();
//     let ordered_bars = (0..count)
//         .map(|k| make_progress_spinner(&mp, format!("[{}/{count}]", k + 1)))
//         .collect::<Vec<_>>();
//     ordered_bars
//         .into_iter()
//         .zip(sources)
//         // Have to `collect()` first because we can't use parallel iterator with Zip
//         .collect::<Vec<_>>()
//         .into_par_iter()
//         .map(|(bar, (n, s))| fetch_one(n, s, out_dir.as_ref(), bar))
//         .collect::<Vec<_>>()
// }
