use fetch_source::{Artefact, CachedSources, MaybeCachedSource, SourceArtefact, Sources};

use MaybeCachedSource::*;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub type FetchResult = Result<SourceArtefact, anyhow::Error>;

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

fn format_success(name: &str, artefact: &SourceArtefact) -> (ProgressStyle, String) {
    let local = match artefact.artefact() {
        Artefact::Git(repo) => repo.local.display(),
        Artefact::Tar(tar) => tar.path.display(),
    };
    (
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} {msg:.cyan/blue}").unwrap(),
        format!("✅  {name} -> {local}"),
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
fn fetch_one<S, P>(
    name: S,
    source: MaybeCachedSource<'_>,
    out_dir: P,
    bar: ProgressBar,
) -> Option<FetchResult>
where
    S: AsRef<str>,
    P: AsRef<std::path::Path>,
{
    bar.enable_steady_tick(std::time::Duration::from_millis(120));
    match source {
        Cached(_, _) => {
            let (style, message) = format_cached(name.as_ref());
            bar.set_style(style);
            bar.finish_with_message(message);
            None
        }
        NotCached(source) => {
            bar.set_message(format!("⏳  {} -> ", name.as_ref()));
            let result = source.fetch(name.as_ref(), &out_dir);
            let (style, message) = match &result {
                Ok(artefact) => format_success(name.as_ref(), artefact),
                Err(_) => format_failure(name.as_ref()),
            };
            bar.set_style(style);
            bar.finish_with_message(message);
            Some(result.map_err(|e| e.into()))
        }
    }
}

// Fetch all sources in parallel with `rayon`. Pair each source with its own progress bar. Using
// ordered bars means the bars are shown in order
pub fn parallel_fetch<P>(sources: Sources, out_dir: P) -> Vec<FetchResult>
where
    P: AsRef<std::path::Path> + Sync,
{
    use rayon::prelude::*;
    let count = sources.len();
    let mp = MultiProgress::new();
    let make_bar = progress_bar_cb(&mp);
    (0..count)
        .zip(sources)
        // Have to `collect()` first because we can't use parallel iterator with Zip
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|(k, (name, source))| {
            fetch_one(
                name,
                NotCached(source),
                out_dir.as_ref(),
                make_bar(format!("[{}/{count}]", k + 1)),
            )
        })
        .collect::<Vec<_>>()
}

// Same as `parallel_fetch`, but only fetches sources that are not cached.
pub fn parallel_fetch_uncached<'a, P, S>(
    sources: CachedSources<'a, S>,
    out_dir: P,
) -> Vec<FetchResult>
where
    P: AsRef<std::path::Path> + Sync,
    S: AsRef<str> + Sync + Send,
{
    use rayon::prelude::*;
    let count = sources.len();
    let mp = MultiProgress::new();
    let make_bar = progress_bar_cb(&mp);
    (0..count)
        .zip(sources)
        // Have to `collect()` first because we can't use parallel iterator with Zip
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|(k, (name, maybe_cached_source))| {
            fetch_one(
                name,
                maybe_cached_source,
                out_dir.as_ref(),
                make_bar(format!("[{}/{count}]", k + 1)),
            )
        })
        .collect::<Vec<_>>()
}
