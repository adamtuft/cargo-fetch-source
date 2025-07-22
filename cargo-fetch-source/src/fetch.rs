use fetch_source::{Artefact, Source, Sources};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub type FetchResult = Result<Artefact, anyhow::Error>;

fn make_progress_spinner(m: &MultiProgress, prefix: String) -> ProgressBar {
    let pb = m.add(ProgressBar::new_spinner());
    pb.set_style(ProgressStyle::default_spinner());
    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template(
            "{prefix:.cyan.bold/blue.bold} {elapsed:.cyan.dim} {msg:.cyan/blue}",
        )
        .unwrap(),
    );
    pb.set_prefix(prefix);
    pb
}

// Fetch a single source, reporting progress in the provided progress bar
fn fetch_one<S, P>(name: S, source: Source, out_dir: P, bar: ProgressBar) -> FetchResult
where
    S: AsRef<str>,
    P: AsRef<std::path::Path>,
{
    bar.set_message(format!("🔄 Fetching {}...", name.as_ref()));
    let result = source.fetch(name.as_ref(), out_dir.as_ref());
    let status = match result {
        Ok(Artefact::Git(ref repo)) => {
            format!("✅ Cloned repository into {}", repo.local.display())
        }
        Ok(Artefact::Tar(ref tar)) => {
            format!(
                "✅ Extracted {} into {}",
                tar.url,
                out_dir.as_ref().display()
            )
        }
        Err(_) => {
            bar.set_style(
                ProgressStyle::with_template(
                    "{prefix:.cyan.bold/blue.bold} {elapsed:.cyan.dim} {msg:.red.bold}",
                )
                .unwrap(),
            );
            format!("⚠️ Failed to fetch '{}'", name.as_ref())
        }
    };
    bar.finish_with_message(status);
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
