use fetch_source::{Artefact, FetchResult, NamedSourceArtefact, Source};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

fn progress_bar_cb<'a>(mp: &'a MultiProgress) -> impl Fn(String) -> ProgressBar + 'a {
    move |prefix: String| {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::with_template(
                "{prefix:.cyan.bold/blue.bold} {msg:.cyan/blue} {spinner}",
            )
            .unwrap()
            .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
        );
        pb.set_prefix(prefix);
        pb
    }
}

fn format_success(artefact: &NamedSourceArtefact) -> (ProgressStyle, String) {
    let local = match artefact.artefact.artefact() {
        Artefact::Git(repo) => repo.local.display(),
        Artefact::Tar(tar) => tar.path.display(),
    };
    (
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} {msg:.cyan/blue}").unwrap(),
        format!("✅  {} -> {}", &artefact.name, local),
    )
}

fn format_failure(name: &str) -> (ProgressStyle, String) {
    (
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} {msg:.red.bold}").unwrap(),
        format!("⚠️  failed to fetch '{name}'"),
    )
}

fn format_cached(name: &str) -> (ProgressStyle, String) {
    (
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} {msg:.cyan/blue}").unwrap(),
        format!("📦  {name} -> <cached>"),
    )
}

// Fetch a single source, reporting progress in the provided progress bar
fn fetch_one(
    name: String,
    source: Source,
    bar: ProgressBar,
    artefact_path: std::path::PathBuf,
) -> FetchResult {
    bar.enable_steady_tick(std::time::Duration::from_millis(120));
    bar.set_message(format!("⏳  {name} -> "));
    let result = source.fetch(name, &artefact_path);
    let (style, message) = match &result {
        Ok(artefact) => format_success(&artefact),
        Err(err) => format_failure(&err.name),
    };
    bar.set_style(style);
    bar.finish_with_message(message);
    result
}

// Fetch all sources in parallel with `rayon`. Pair each source with its own progress bar.
pub fn fetch_all_parallel(sources: Vec<(String, Source, std::path::PathBuf)>) -> Vec<FetchResult> {
    use rayon::prelude::*;
    let count = sources.len();
    let mp = MultiProgress::new();
    let make_bar = progress_bar_cb(&mp);
    sources
        .into_par_iter()
        .enumerate()
        .map(|(k, (name, source, artefact_path))| {
            let bar = make_bar(format!("[{}/{}]", k + 1, count));
            fetch_one(name, source, bar, artefact_path)
        })
        .collect()
}
