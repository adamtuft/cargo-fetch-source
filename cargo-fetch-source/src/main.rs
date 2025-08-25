use crate::{
    error::{AppError, AppErrorKind},
    fetch::fetch_all_parallel,
};
use fetch_source::{Source, SourcesTable};
use std::error::Error;

mod args;
mod error;
mod fetch;

use args::OutputFormat;

fn main() -> std::process::ExitCode {
    if let Err(err) = run() {
        match err.error_kind() {
            // Fetch errors are reported inside run(), so just convert error to exit code
            AppErrorKind::Fetch => {}
            _ => eprintln!("{err}"),
        }
        err.into()
    } else {
        std::process::ExitCode::from(0)
    }
}

fn sources(manifest_file: &std::path::Path) -> Result<fetch_source::SourcesTable, error::AppError> {
    let document = std::fs::read_to_string(manifest_file)
        .map_err(|err| AppError::manifest_read(format!("{}", manifest_file.display()), err))?;

    fetch_source::try_parse_toml(&document)
        .map_err(|err| AppError::manifest_parse(format!("{}", manifest_file.display()), err))
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
            let sources = sources(&manifest_file)?;
            let num_sources = sources.len();
            let (artefacts, errors) = fetch_and_cache_sources(sources, cache_items, &cache_dir);
            cache.save().map_err(|err| {
                AppError::cache_save_failed(cache.cache_file().to_path_buf(), err)
            })?;
            copy_all_artefacts(&out_dir, artefacts)?;

            // Report errors and return error status if any occurred
            if errors.is_empty() {
                Ok(())
            } else {
                report_fetch_results(errors, num_sources);
                Err(AppError::fetch())
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

// Fetch missing sources and return all the now-cached sources, and errors for those which couldn't
// be fetched
fn fetch_and_cache_sources(
    sources: SourcesTable,
    cache_items: &mut fetch_source::CacheItems,
    cache_root: &fetch_source::CacheRoot,
) -> (
    Vec<(String, fetch_source::CacheDir)>,
    Vec<fetch_source::FetchError>,
) {
    let (cached, missing) = sources
        .into_iter()
        .partition(|(_, s)| cache_items.contains(s));
    let (mut fetched, errors) = fetch_all_parallel(missing, cache_items, cache_root);

    // Combine the newly-fetched with the previously-cached artefacts. Drop the source values as
    // they are now contained in the cached artefacts. Instead, give the path to the cached
    // artefacts
    fetched.extend(
        cached
            .into_iter()
            .map(|(name, source)| (name, cache_root.append(cache_items.relative_path(&source)))),
    );

    (fetched, errors)
}

fn copy_all_artefacts<P>(
    out_dir: P,
    artefacts: Vec<(String, fetch_source::CacheDir)>,
) -> Result<(), AppError>
where
    P: AsRef<std::path::Path>,
{
    for (name, artefact_path) in artefacts {
        copy_artefact(&out_dir, name, &*artefact_path)?;
    }
    Ok(())
}

fn copy_artefact<P, Q>(out_dir: P, name: String, artefact_path: Q) -> Result<(), AppError>
where
    P: AsRef<std::path::Path>,
    Q: AsRef<std::path::Path>,
{
    if !artefact_path.as_ref().is_dir() {
        return Err(AppError::missing_artefact_directory(
            name,
            artefact_path.as_ref().to_path_buf(),
        ));
    }
    let dest = out_dir.as_ref().join(Source::as_path_component(&name));
    dircpy::copy_dir(artefact_path.as_ref(), &dest).map_err(|err| {
        AppError::copy_artefact_failed(artefact_path.as_ref().to_path_buf(), dest, err)
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
                            println!("   branch/tag:  {branch}");
                        } else if let Some(commit) = git.commit_sha() {
                            println!("   commit:  {commit}");
                        }
                        println!("   recursive: {}", git.is_recursive());
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
        serde_json::to_string_pretty(cache.items()).expect("Failed to serialize cache")
    );
    Ok(())
}
