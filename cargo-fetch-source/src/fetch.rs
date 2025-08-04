use fetch_source::{
    Artefact, CacheDir, CacheItems, CacheRoot, FetchError, FetchResult, Source, SourceName,
    SourcesTable,
};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

fn progress_bar(mp: &MultiProgress, prefix: String) -> ProgressBar {
    let pb = mp.add(ProgressBar::new_spinner());
    pb.set_style(
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} {msg:.cyan/blue} {spinner}")
            .unwrap()
            .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
    );
    pb.set_prefix(prefix);
    pb
}

fn format_success(name: &str, artefact: &Artefact) -> (ProgressStyle, String) {
    let path: &std::path::Path = artefact.as_ref();
    (
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} {msg:.cyan/blue}").unwrap(),
        format!("✅  {} -> {}", name, path.display()),
    )
}

fn format_failure(name: &str) -> (ProgressStyle, String) {
    (
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} {msg:.red.bold}").unwrap(),
        format!("⚠️  failed to fetch '{name}'"),
    )
}

// Fetch a single source, reporting progress in the provided progress bar
fn fetch_one(
    name: &str,
    source: Source,
    bar: ProgressBar,
    artefact_path: &CacheDir,
) -> FetchResult<Artefact> {
    bar.enable_steady_tick(std::time::Duration::from_millis(120));
    bar.set_message(format!("⏳  {name} -> "));
    let result = source.fetch(&**artefact_path);
    let (style, message) = match &result {
        Ok(artefact) => format_success(name, artefact),
        Err(_) => format_failure(name),
    };
    bar.set_style(style);
    bar.finish_with_message(message);
    result
}

#[inline]
fn accumulate_fetch_results(
    items: &mut CacheItems,
    mut fetched: Vec<(SourceName, CacheDir)>,
    mut errors: Vec<FetchError>,
    result: Result<(String, Artefact, CacheDir), FetchError>,
) -> (Vec<(SourceName, CacheDir)>, Vec<FetchError>) {
    match result {
        Ok((name, artefact, artefact_path)) => {
            fetched.push((name, artefact_path));
            items.insert(artefact);
        }
        Err(error) => errors.push(error),
    }
    (fetched, errors)
}

// Fetch all sources in parallel with `rayon`. Pair each source with its own progress bar. Insert
// fetched artefacts into cache and return fetched artefact locations and fetch errors.
pub fn fetch_all_parallel(
    sources: SourcesTable,
    cache_items: &mut CacheItems,
    cache_root: &CacheRoot,
) -> (Vec<(SourceName, CacheDir)>, Vec<FetchError>) {
    use rayon::prelude::*;
    let n = std::sync::atomic::AtomicUsize::new(0);
    let count = sources.len();
    let mp = MultiProgress::new();
    sources
        .into_par_iter()
        // Perform the fetch in parallel
        .map(|(name, source)| {
            let k = n.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let bar = progress_bar(&mp, format!("[{}/{}]", k + 1, count));
            let artefact_path = cache_root.append(cache_items.relative_path(&source));
            fetch_one(&name, source, bar, &artefact_path)
                .map(|artefact| (name, artefact, artefact_path))
        })
        .collect::<Vec<_>>()
        .into_iter()
        // Insert fetched artefacts, accumulate errors
        .fold((Vec::new(), Vec::new()), |(f, e), r| {
            accumulate_fetch_results(cache_items, f, e, r)
        })
}
