use crate::{error::AppError, fetch::fetch_all_parallel};
use fetch_source::Source;
use std::error::Error;

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
            manifest_file,
            mut cache,
        } => {
            let cache_dir = cache.cache_dir();
            let cache_items = cache.items_mut();
            let (cached, err) = fetch(sources(&manifest_file)?, cache_dir, cache_items);
            cache.save().map_err(|err| AppError::CacheSaveFailed {
                path: cache.cache_file().to_path_buf(),
                err,
            })?;
            for (name, artefact_path) in cached {
                copy_artefact(&out_dir, name, artefact_path)?;
            }
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
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
    sources: fetch_source::SourcesTable,
    cache_dir: fetch_source::CacheDir,
    cache_items: &mut fetch_source::CacheItems,
) -> (Vec<(String, fetch_source::ArtefactPath)>, Option<AppError>) {
    let num_sources = sources.len();
    let (cached, errors) =
        cache_items.fetch_missing(sources.into_iter(), cache_dir, fetch_all_parallel);
    if errors.is_empty() {
        (cached, None)
    } else {
        report_fetch_results(errors, num_sources);
        (cached, Some(AppError::Fetch))
    }
}

fn copy_artefact<P>(
    out_dir: &std::path::Path,
    name: String,
    artefact_path: P,
) -> Result<(), AppError>
where
    P: AsRef<std::path::Path>,
{
    if !artefact_path.as_ref().is_dir() {
        return Err(AppError::MissingArtefactDirectory {
            name,
            path: artefact_path.as_ref().to_path_buf(),
        });
    }
    let dest = out_dir.join(Source::as_path_component(&name));
    println!("{name}: COPY {:#?} -> {dest:#?}", artefact_path.as_ref());
    dircpy::copy_dir(artefact_path.as_ref(), &dest).map_err(|err| {
        AppError::CopyArtefactFailed {
            src: artefact_path.as_ref().to_path_buf(),
            dst: dest,
            err,
        }
    })?;
    Ok(())
}

// Report fetch results, including any errors and success messages.
fn report_fetch_results(errors: Vec<fetch_source::FetchError>, num_sources: usize) {
    let num_errors = errors.len();
    let num_success = num_sources - num_errors;
    let error_style = console::Style::new().red().bold();
    eprintln!("Failed to fetch {} sources:", errors.len());
    for (k, err) in (1..).zip(&errors) {
        eprintln!(
            "Error [{k}/{num_errors}]: {}",
            error_style.apply_to(err.to_string())
        );
        let mut error_source = err.source();
        while let Some(cause) = error_source {
            let cause_text = format!("{cause}");
            let mut line_iter = cause_text.split("\n");
            eprintln!(
                "  caused by: {}",
                error_style.apply_to(line_iter.next().unwrap_or("?"))
            );
            line_iter.for_each(|line| eprintln!("             {}", error_style.apply_to(line)));
            error_source = cause.source();
        }
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
