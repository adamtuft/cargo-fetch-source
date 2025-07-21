use fetch_source::{Artefact, Source, Sources};

pub type FetchResult = Result<Artefact, anyhow::Error>;

fn make_progress_spinner(m: &indicatif::MultiProgress, prefix: String) -> indicatif::ProgressBar {
    let pb = m.add(indicatif::ProgressBar::new_spinner());
    pb.set_style(indicatif::ProgressStyle::default_spinner());
    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb.set_style(
        indicatif::ProgressStyle::with_template(
            "{prefix:.cyan.bold/blue.bold} {elapsed:.cyan.dim} {msg:.cyan/blue}",
        )
        .unwrap(),
    );
    pb.set_prefix(prefix);
    pb
}

fn fetch_one<S: AsRef<str>>(
    name: S,
    source: Source,
    out_dir: &std::path::Path,
    bar: indicatif::ProgressBar,
) -> FetchResult {
    bar.set_message(format!("🔄 Fetching {}...", name.as_ref()));
    let result = source.fetch(name.as_ref(), out_dir);
    let status = match result {
        Ok(Artefact::Git(ref repo)) => {
            format!("✅ Cloned repository into {}", repo.local.display())
        }
        Ok(Artefact::Tar(ref tar)) => {
            format!("✅ Extracted {} into {}", tar.url, out_dir.display())
        }
        Err(_) => {
            bar.set_style(
                indicatif::ProgressStyle::with_template(
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

pub fn parallel_fetch(sources: Sources, out_dir: &std::path::Path) -> Vec<FetchResult> {
    use rayon::prelude::*;
    let count = sources.len();
    let mp = indicatif::MultiProgress::new();
    let ordered_bars = (0..count)
        .map(|k| make_progress_spinner(&mp, format!("[{}/{count}]", k + 1)))
        .collect::<Vec<_>>();
    ordered_bars
        .into_iter()
        .zip(sources)
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|(bar, (n, s))| fetch_one(n, s, out_dir, bar))
        .collect::<Vec<_>>()
}
