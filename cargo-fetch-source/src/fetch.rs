use fetch_source::{
    Artefact, CacheDir, CacheItems, CacheRoot, FetchError, SourceName, SourcesTable,
};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

fn format_process(name: &str) -> (ProgressStyle, String) {
    (
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} {msg:.cyan/blue} {spinner}")
            .unwrap()
            .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
        format!("⏳  {name} -> "),
    )
}

fn format_success(name: &str, cache_dir: &CacheDir) -> (ProgressStyle, String) {
    (
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} {msg:.cyan/blue}").unwrap(),
        format!("✅  {} -> {}", name, cache_dir.display()),
    )
}

fn format_failure(name: &str) -> (ProgressStyle, String) {
    (
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} {msg:.red.bold}").unwrap(),
        format!("⚠️  failed to fetch '{name}'"),
    )
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
            let (style, message) = format_process(&name);
            let bar = mp.add(ProgressBar::new_spinner());
            bar.set_style(style);
            bar.set_message(message);
            bar.set_prefix(format!("[{}/{}]", k + 1, count));
            bar.enable_steady_tick(std::time::Duration::from_millis(120));
            let artefact_path = cache_root.append(cache_items.relative_path(&source));
            let result = source.fetch(&**artefact_path);
            let (style, message) = if result.is_ok() {
                format_success(&name, &artefact_path)
            } else {
                format_failure(&name)
            };
            bar.set_style(style);
            bar.finish_with_message(message);
            result.map(|artefact| (name, artefact, artefact_path))
        })
        .collect::<Vec<_>>()
        .into_iter()
        // Insert fetched artefacts, accumulate errors
        .fold((Vec::new(), Vec::new()), |(f, e), r| {
            accumulate_fetch_results(cache_items, f, e, r)
        })
}
