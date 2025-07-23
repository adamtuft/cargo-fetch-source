use fetch_source::{Artefact, Source, SourceArtefact, Sources};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub type FetchResult = Result<SourceArtefact, anyhow::Error>;

fn make_progress_spinner(m: &MultiProgress, prefix: String) -> ProgressBar {
    let pb = m.add(ProgressBar::new_spinner());
    // pb.set_style(ProgressStyle::default_spinner());
    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{prefix:.cyan.bold/blue.bold} 🔎 {msg:.cyan/blue} {spinner}")
            .unwrap()
            .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
    );
    pb.set_prefix(prefix);
    pb
}

fn complete_progress_bar(pb: ProgressBar, result: &Result<SourceArtefact, fetch_source::Error>) {
    let template = if result.is_ok() {
        "{prefix:.cyan.bold/blue.bold} {msg:.cyan/blue}"
    } else {
        "{prefix:.cyan.bold/blue.bold} {msg:.red.bold}"
    };
    pb.set_style(ProgressStyle::with_template(template).unwrap());
    let status = if let Ok(fetched) = &result {
        match fetched.artefact() {
            Artefact::Git(repo) => format!("😸 {} -> {}", pb.prefix(), repo.local.display()),
            Artefact::Tar(tar) => format!("😸 {} -> {}", pb.prefix(), tar.path.display()),
        }
    } else {
        format!("😿 failed to fetch '{}'", pb.prefix())
    };
    pb.finish_with_message(status);
}

// Fetch a single source, reporting progress in the provided progress bar
fn fetch_one<S, P>(name: S, source: Source, out_dir: P, bar: ProgressBar) -> FetchResult
where
    S: AsRef<str>,
    P: AsRef<std::path::Path>,
{
    bar.set_message(format!("{} -> ", name.as_ref()));
    let result = source.fetch(name.as_ref(), &out_dir);
    complete_progress_bar(bar, &result);
    Ok(result?)
}

// Fetch sources in parallel with `rayon`. Pair each source with its own progress bar. Using
// ordered bars means the bars are shown in order
pub fn parallel_fetch<P>(sources: Sources, out_dir: P) -> Vec<FetchResult>
where
    P: AsRef<std::path::Path> + Sync,
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
        .map(|(bar, (n, s))| fetch_one(n, s, out_dir.as_ref(), bar))
        .collect::<Vec<_>>()
}
