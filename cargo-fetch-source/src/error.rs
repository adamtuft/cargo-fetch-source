use std::process::ExitCode;

use thiserror::Error;

#[derive(Debug, Error)]
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
}

impl From<AppError> for ExitCode {
    fn from(error: AppError) -> Self {
        ExitCode::from(match error {
            AppError::Fetch => 1,
            AppError::ArgValidation(_) => 2,
            AppError::IO(_) => 3,
            AppError::ManifestRead {
                manifest: _,
                err: _,
            } => 4,
            AppError::ManifestParse {
                manifest: _,
                err: _,
            } => 5,
        })
    }
}
