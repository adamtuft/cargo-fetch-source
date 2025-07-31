use fetch_source::{Artefact, FetchResult, Source, SourceName, ArtefactPath};

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
    artefact_path: &std::path::Path,
) -> FetchResult<Artefact> {
    bar.enable_steady_tick(std::time::Duration::from_millis(120));
    bar.set_message(format!("⏳  {name} -> "));
    let result = source.fetch(artefact_path);
    let (style, message) = match &result {
        Ok(artefact) => format_success(name, artefact),
        Err(_) => format_failure(name),
    };
    bar.set_style(style);
    bar.finish_with_message(message);
    result
}

// Fetch all sources in parallel with `rayon`. Pair each source with its own progress bar.
pub fn fetch_all_parallel(
    sources: Vec<(SourceName, Source, ArtefactPath)>,
) -> Vec<FetchResult<(SourceName, Artefact)>> {
    use rayon::prelude::*;
    let count = sources.len();
    let mp = MultiProgress::new();
    let make_bar = progress_bar_cb(&mp);
    sources
        .into_par_iter()
        .enumerate()
        .map(|(k, (name, source, artefact_path))| {
            let bar = make_bar(format!("[{}/{}]", k + 1, count));
            fetch_one(&name, source, bar, artefact_path.as_ref()).map(|artefact| (name, artefact))
        })
        .collect()
}
