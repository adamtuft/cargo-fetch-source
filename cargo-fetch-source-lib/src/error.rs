// use crate::git2_ext;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error("Manual error: {0}")]
    Manual(String),
    #[error("Command '{command}' exited with status {status}\n{stderr}")]
    Subprocess{
        command: String,
        status: std::process::ExitStatus,
        stderr: String,
    }
}
