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
        stderr: String,
    ) -> Self {
        Self {
            inner: ErrorKind::Subprocess {
                command,
                status,
                stderr,
            },
        }
    }
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
    Parse(#[from] crate::SourceParseError),

    #[error("Command '{command}' exited with status {status}\n{stderr}")]
    Subprocess {
        command: String,
        status: std::process::ExitStatus,
        stderr: String,
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
