use crate::{error::AppError, fetch::parallel_fetch};

mod args;
mod error;
mod fetch;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::from(0),
        Err(err) => {
            match err {
                AppError::Fetch(_) => {}
                _ => eprintln!("{err}"),
            }
            err.into()
        }
    }
}

fn run() -> Result<(), error::AppError> {
    let args = args::parse()?;

    // SAFETY: This is called before any thread-spawning constructs are encountered, so there is
    // definitely only one thread active.
    if let Some(threads) = args.threads {
        unsafe { std::env::set_var("RAYON_NUM_THREADS", format!("{threads}")) };
    }

    let document =
        std::fs::read_to_string(&args.manifest_file).map_err(|err| AppError::ManifestRead {
            manifest: format!("{}", args.manifest_file.display()),
            err,
        })?;

    let sources =
        fetch_source::try_parse_toml(&document).map_err(|err| AppError::ManifestParse {
            manifest: format!("{}", args.manifest_file.display()),
            err,
        })?;

    match args.action {
        args::Action::Fetch => fetch(sources, &args.out_dir),
        args::Action::List => list(sources, &args.out_dir),
    }
}

fn fetch(sources: fetch_source::Sources, out_dir: &std::path::Path) -> Result<(), error::AppError> {
    let num_sources = sources.len();
    let errors: Vec<_> = parallel_fetch(sources, out_dir)
        .into_iter()
        .filter_map(Result::err)
        .collect();
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

    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::Fetch(num_errors))
    }
}

fn list(sources: fetch_source::Sources, _: &std::path::Path) -> Result<(), error::AppError> {
    // for (name, source) in sources {
    //     println!("{name}:");
    //     match source {
    //         fetch_source::Source::Tar(tar) => {
    //             println!("   upstream: {}", tar.upstream());
    //         },
    //         fetch_source::Source::Git(git) => {
    //             println!("   upstream: {}", git.upstream());
    //             if let Some(branch) = git.branch_name() {
    //                 println!("  branch/tag:  {branch}");
    //             } else if let Some(commit) = git.commit_sha() {
    //                 println!("  commit:  {commit}");
    //             }
    //             println!("  recursive: {}", git.is_recursive());
    //         },
    //     }
    // }
    
    // Use serde to serialise `sources` to TOML, then print it.
    // SAFETY: unwrap here because we only accept values that were previously deserialised
    let toml = toml::to_string(&sources).unwrap();
    println!("{toml}");

    Ok(())
}
