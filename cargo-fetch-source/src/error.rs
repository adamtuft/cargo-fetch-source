use std::process::ExitCode;

/// Internal error type that contains all application error variants.
/// This type has private fields and is boxed to reduce stack size.
#[derive(Debug, thiserror::Error)]
pub enum AppErrorInner {
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
    #[error("Failed to fetch one or more source(s)")]
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

/// The main application-level error type. This represents all top-level application errors we'd
/// want to report to the user. We don't use `anyhow::{Context, Error}` for this as we still care
/// about the concrete error type (for the exit code) while wanting to add additional context to the
/// error, such as what the application was doing when an IO error happened (e.g. reading the
/// manifest file vs. copying an artefact directory). In particular, the `Fetch` variant is only
/// used to indicate that errors occurred during fetching - these errors are reported immediately
/// rather than being returned, so this variant only exists to produce the correct `ExitCode`.
/// 
/// This type uses the newtype pattern to wrap a boxed inner error, reducing stack size.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct AppError(Box<AppErrorInner>);

impl std::ops::Deref for AppError {
    type Target = AppErrorInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AppError {
    /// Create a new AppError with the given inner error
    pub fn new(inner: AppErrorInner) -> Self {
        Self(Box::new(inner))
    }

    /// Create an argument validation error
    pub fn arg_validation(msg: String) -> Self {
        Self::new(AppErrorInner::ArgValidation(msg))
    }

    /// Create a manifest read error
    pub fn manifest_read(manifest: String, err: std::io::Error) -> Self {
        Self::new(AppErrorInner::ManifestRead { manifest, err })
    }

    /// Create a manifest parse error
    pub fn manifest_parse(manifest: String, err: fetch_source::SourceParseError) -> Self {
        Self::new(AppErrorInner::ManifestParse { manifest, err })
    }

    /// Create a fetch error
    pub fn fetch() -> Self {
        Self::new(AppErrorInner::Fetch)
    }

    /// Create a copy artefact failed error
    pub fn copy_artefact_failed(src: std::path::PathBuf, dst: std::path::PathBuf, err: std::io::Error) -> Self {
        Self::new(AppErrorInner::CopyArtefactFailed { src, dst, err })
    }

    /// Create a cache save failed error
    pub fn cache_save_failed(path: std::path::PathBuf, err: fetch_source::Error) -> Self {
        Self::new(AppErrorInner::CacheSaveFailed { path, err })
    }

    /// Create a missing artefact directory error
    pub fn missing_artefact_directory(name: String, path: std::path::PathBuf) -> Self {
        Self::new(AppErrorInner::MissingArtefactDirectory { name, path })
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::new(AppErrorInner::IO(err))
    }
}

impl From<AppError> for ExitCode {
    fn from(error: AppError) -> Self {
        ExitCode::from(match &*error {
            AppErrorInner::Fetch => 1,
            AppErrorInner::ArgValidation(_) => 2,
            _ => 3,
        })
    }
}
