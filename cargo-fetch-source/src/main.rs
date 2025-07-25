use crate::{error::AppError, fetch::parallel_fetch_uncached};

mod args;
mod error;
mod fetch;

use args::OutputFormat;

fn main() -> std::process::ExitCode {
    if let Err(err) = run() {
        match err {
            // Fetch errors are reported inside run(), so just convert error to exit code
            AppError::Fetch => {}
            _ => eprintln!("{err}"),
        }
        err.into()
    } else {
        std::process::ExitCode::from(0)
    }
}

fn sources(manifest_file: &std::path::Path) -> Result<fetch_source::SourcesTable, error::AppError> {
    let document =
        std::fs::read_to_string(manifest_file).map_err(|err| AppError::ManifestRead {
            manifest: format!("{}", manifest_file.display()),
            err,
        })?;

    fetch_source::try_parse_toml(&document).map_err(|err| AppError::ManifestParse {
        manifest: format!("{}", manifest_file.display()),
        err,
    })
}

fn run() -> Result<(), error::AppError> {
    let args = args::parse()?;

    match args.command {
        args::ValidatedCommand::Fetch {
            out_dir,
            threads,
            manifest_file,
            cache,
        } => fetch(&out_dir, threads, &manifest_file, cache),
        args::ValidatedCommand::List {
            format,
            manifest_file,
        } => list(sources(&manifest_file)?, format),
        args::ValidatedCommand::Cached {
            format: _,
            ref cache,
        } => cached(cache),
    }
}

fn fetch(
    out_dir: &std::path::Path,
    threads: Option<u32>,
    manifest_file: &std::path::Path,
    cache: fetch_source::Cache,
) -> Result<(), AppError> {
    let sources = sources(manifest_file)?;
    let num_sources = sources.len();
    match fetch_sources(sources, out_dir, cache, threads)? {
        errors if errors.is_empty() => Ok(()),
        errors => {
            report_fetch_results(errors, num_sources);
            Err(AppError::Fetch)
        }
    }
}

// Fetch all sources and return any errors that occurred during fetching.
// Update the cache with any successfully fetched artefacts.
fn fetch_sources(
    sources: fetch_source::SourcesTable,
    out_dir: &std::path::Path,
    mut cache: fetch_source::Cache,
    threads: Option<u32>,
) -> Result<Vec<anyhow::Error>, AppError> {
    if let Some(threads) = threads {
        // SAFETY: only called in a serial region before any other threads exist.
        unsafe { std::env::set_var("RAYON_NUM_THREADS", format!("{threads}")) };
    }

    let cached_sources = cache.into_cached_sources(sources.clone());
    let fetch_results = parallel_fetch_uncached(&cached_sources, &cache);

    let mut errors = Vec::new();
    let mut fetched_artefacts = Vec::new();
    for result in fetch_results {
        match result {
            Ok(artefact) => fetched_artefacts.push(artefact),
            Err(err) => errors.push(err),
        }
    }

    for artefact in fetched_artefacts {
        cache.insert(artefact);
    }

    // Copy all cached sources to the output directory
    for (name, source) in &sources {
        let digest = fetch_source::Cache::digest(source);
        let artefact_path = cache.artefact_path(&digest);
        if artefact_path.is_dir() {
            dircpy::copy_dir(artefact_path, &out_dir)
                .map_err(|err| AppError::Cache(format!("failed to copy to output dir"), err.into()))?;
        } else {
            return Err(AppError::Cache(
                format!("artefact for digest {digest} not found"),
                fetch_source::CacheEntryNotFound { digest }.into(),
            ));
        }
    }

    cache.save().map_err(|err| {
        AppError::Cache(
            format!("failed to save cache to {}", cache.cache_file().display()),
            err,
        )
    })?;

    Ok(errors)
}

// Report fetch results, including any errors and success messages.
fn report_fetch_results(errors: Vec<anyhow::Error>, num_sources: usize) {
    let num_errors = errors.len();
    let num_success = num_sources - num_errors;
    let error_style = console::Style::new().red().bold();
    eprintln!("Failed to fetch {} sources:", errors.len());
    for (k, err) in (1..).zip(&errors) {
        eprintln!(
            "Error [{k}/{num_errors}]: {}",
            error_style.apply_to(err.to_string())
        );
        err.chain().skip(1).for_each(|cause| {
            let cause_text = format!("{cause}");
            let mut line_iter = cause_text.split("\n");
            eprintln!(
                "  caused by: {}",
                error_style.apply_to(line_iter.next().unwrap_or("?"))
            );
            line_iter.for_each(|line| eprintln!("             {}", error_style.apply_to(line)));
        });
    }
    if num_success > 0 {
        println!("🎉 Successfully fetched {num_success} source(s)!");
    }
}

// List all sources in the chosen format
fn list(sources: fetch_source::SourcesTable, format: Option<OutputFormat>) -> Result<(), AppError> {
    match format {
        Some(OutputFormat::Toml) => {
            // SAFETY: unwrap here because we only accept values that were previously deserialised
            println!("{}", toml::to_string(&sources).unwrap());
        }
        Some(OutputFormat::Json) => {
            // SAFETY: unwrap here because we only accept values that were previously deserialised
            println!("{}", serde_json::to_string_pretty(&sources).unwrap());
        }
        None => {
            for (name, source) in sources {
                println!("{name}:");
                match source {
                    fetch_source::Source::Tar(tar) => {
                        println!("   upstream: {}", tar.upstream());
                    }
                    fetch_source::Source::Git(git) => {
                        println!("   upstream: {}", git.upstream());
                        if let Some(branch) = git.branch_name() {
                            println!("  branch/tag:  {branch}");
                        } else if let Some(commit) = git.commit_sha() {
                            println!("  commit:  {commit}");
                        }
                        println!("  recursive: {}", git.is_recursive());
                    }
                }
            }
        }
    }
    Ok(())
}

fn cached(cache: &fetch_source::Cache) -> Result<(), AppError> {
    println!(
        "// Contents of cache file: {}",
        cache.cache_file().display()
    );
    println!(
        "{}",
        serde_json::to_string_pretty(cache).expect("Failed to serialize cache")
    );
    Ok(())
}
