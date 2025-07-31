//! Defines the error type for this crate.

/// Errors that occur during fetching
#[derive(Debug, thiserror::Error)]
#[error("failed to fetch source")]
pub struct FetchError {
    source: crate::Source,
    #[source]
    err: FetchErrorKind,
}

impl FetchError {
    pub(crate) fn new(err: FetchErrorKind, source: crate::Source) -> Self {
        Self { source, err }
    }
}

/// Internal error categories.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FetchErrorKind {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error("subprocess '{command}' exited with status {status}\n{stderr}")]
    Subprocess {
        command: String,
        status: std::process::ExitStatus,
        stderr: String,
    },
}

impl FetchErrorKind {
    pub fn subprocess(command: String, status: std::process::ExitStatus, stderr: String) -> Self {
        Self::Subprocess {
            command,
            status,
            stderr,
        }
    }
}

/// The main error type for this crate.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error {
    inner: ErrorKind,
}

/// Internal error categories.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ErrorKind {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),

    #[error(transparent)]
    SerdeDe(#[from] serde_json::Error),

    #[error(transparent)]
    Parse(#[from] crate::SourceParseError),
}

// Blanket implementation for all variants of ErrorKind with a #[from] attribute
impl<T> From<T> for Error
where
    ErrorKind: From<T>,
{
    fn from(e: T) -> Self {
        Self {
            inner: ErrorKind::from(e),
        }
    }
}
