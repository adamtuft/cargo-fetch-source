//! Defines the error type for this crate.

/// The main error type for this crate.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error {
    inner: ErrorKind,
}

impl Error {
    /// Manual constructor for a subprocess error. This exists because there's no lower error type
    /// to forward.
    pub(crate) fn subprocess(
        command: String,
        status: std::process::ExitStatus,
        cause: anyhow::Error,
    ) -> Self {
        Self {
            inner: ErrorKind::Subprocess {
                command,
                status,
                cause,
            },
        }
    }
}



/// A required entry was not found in the cache
#[derive(Debug, thiserror::Error)]
#[error("cache entry for digest '{digest}' not found")]
pub struct CacheEntryNotFound {
    pub digest: String,
}

/// Internal error categories.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ErrorKind {
    #[error(transparent)]
    CacheEntryNotFound(#[from] CacheEntryNotFound),

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

    #[error("Command '{command}' exited with status {status}")]
    Subprocess {
        command: String,
        status: std::process::ExitStatus,
        #[source]
        cause: anyhow::Error,
    },
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
