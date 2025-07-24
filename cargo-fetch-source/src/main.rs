use crate::{
    error::AppError,
    fetch::parallel_fetch_uncached,
};

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

fn sources(manifest_file: &std::path::Path) -> Result<fetch_source::Sources, error::AppError> {
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
            mut cache,
        } => {
            if let Some(threads) = threads {
                // SAFETY: only called in a serial region before any other threads exist.
                unsafe { std::env::set_var("RAYON_NUM_THREADS", format!("{threads}")) };
            }
            let outcome = fetch(sources(&manifest_file)?, &out_dir, &mut cache);
            cache.save().map_err(|err| {
                assert!(cache.cache_file() != std::ffi::OsStr::new(""), "Cache file should be set");
                AppError::Cache(
                    format!("failed to save cache to {}", cache.cache_file().display()),
                    err,
                )
            })?;
            if outcome {
                Ok(())
            } else {
                // `fetch` returns false on failure, so report an error to produce exit code
                Err(AppError::Fetch)
            }
        }
        args::ValidatedCommand::List {
            format,
            manifest_file,
        } => {
            list(sources(&manifest_file)?, format);
            Ok(())
        }
        args::ValidatedCommand::Cached {
            format: _,
            ref cache,
        } => {
            println!("// Contents of cache file: {}", cache.cache_file().display());
            println!("{}", serde_json::to_string_pretty(cache).expect("Failed to serialize cache"));
            Ok(())
        }
    }
}

// Fetch all sources and report outcome with progress bars. Report any errors fetching sources.
// All sub-errors are swallowed and reported here so just bool to indicate success/failure.
// Update the cache with any successfully fetched artefacts.
fn fetch(
    sources: fetch_source::Sources,
    out_dir: &std::path::Path,
    cache: &mut fetch_source::Cache,
) -> bool {
    let num_sources = sources.len();

    // Fetch sources then fold the fetched sources into the cache. Accumulate any errors.
    let (errors, _) = parallel_fetch_uncached(cache.into_cached_sources(sources), out_dir)
        .into_iter()
        .fold((Vec::new(), cache), |(mut v, c), result| {
            match result {
                Ok(artefact) => {
                    c.insert(artefact);
                }
                Err(err) => {
                    v.push(err);
                }
            }
            (v, c)
        });

    let num_errors = errors.len();

    if !errors.is_empty() {
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
    }

    let num_success = num_sources - num_errors;
    if num_success > 0 {
        println!("🎉 Successfully fetched {num_success} source(s)!");
    }

    num_errors == 0
}

// List all sources in the chosen format
fn list(sources: fetch_source::Sources, format: Option<OutputFormat>) {
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
}
