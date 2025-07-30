use std::process::ExitCode;

/// The main application-level error type. This represents all top-level application errors we'd
/// want to report to the user. We don't use `anyhow::{Context, Error}` for this as we still care
/// about the concrete error type (for the exit code) while wanting to add additional context to the
/// error, such as what the application was doing when an IO error happened (e.g. reading the
/// manifest file vs. copying an artefact directory). In particular, the `Fetch` variant is only
/// used to indicate that errors occurred during fetching - these errors are reported immediately
/// rather than being returned, so this variant only exists to produce the correct `ExitCode`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Argument error: {0}")]
    ArgValidation(String),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("Failed to read manifest file: {manifest}")]
    ManifestRead {
        manifest: String,
        #[source]
        err: std::io::Error,
    },
    #[error("Failed to parse manifest file: {manifest}")]
    ManifestParse {
        manifest: String,
        #[source]
        err: fetch_source::SourceParseError,
    },
    #[error("Failed to fetch source(s)")]
    Fetch,
    #[error("failed to copy {} to {}", src.display(), dst.display())]
    CopyArtefactFailed {
        src: std::path::PathBuf,
        dst: std::path::PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("failed to save cache to {}", path.display())]
    CacheSaveFailed {
        path: std::path::PathBuf,
        #[source]
        err: fetch_source::Error,
    },
    #[error("expected directory for '{}' to exist at {}", name, path.display())]
    MissingArtefactDirectory {
        name: String,
        path: std::path::PathBuf,
    },
}

impl From<AppError> for ExitCode {
    fn from(error: AppError) -> Self {
        ExitCode::from(match error {
            AppError::Fetch => 1,
            AppError::ArgValidation(_) => 2,
            _ => 3,
        })
    }
}
