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
pub struct Error {
    kind: ErrorKind,
    inner: ErrorImpl,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl Error {
    /// Get the kind of error that occurred
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

/// The different kinds of error that can be emitted by this crate.
#[derive(Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// An I/O error occurred
    Io,
    #[cfg(feature = "reqwest")]
    /// An `Reqwest` error occurred
    Reqwest,
    /// A TOML deserialisation error occurred
    TomlDe,
    /// A serde deserialisation error occurred
    SerdeDe,
    /// An error occurred while parsing sources
    Parse,
}

/// Internal error categories.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ErrorImpl {
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

impl ErrorImpl {
    fn into_error<T>(value: T) -> Error
    where
        ErrorImpl: From<T>,
    {
        let inner = ErrorImpl::from(value);
        let kind = match &inner {
            Self::Io(_) => ErrorKind::Io,
            #[cfg(feature = "reqwest")]
            Self::Reqwest(_) => ErrorKind::Reqwest,
            Self::TomlDe(_) => ErrorKind::TomlDe,
            Self::SerdeDe(_) => ErrorKind::SerdeDe,
            Self::Parse(_) => ErrorKind::Parse,
        };
        Error { kind, inner }
    }
}

// Blanket implementation for all variants of ErrorKind with a #[from] attribute
impl<T> From<T> for Error
where
    ErrorImpl: From<T>,
{
    fn from(err: T) -> Self {
        ErrorImpl::into_error(err)
    }
}
