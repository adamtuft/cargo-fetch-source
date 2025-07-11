use crate::git2_ext;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    Git2Error(#[from] git2_ext::Error),
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error("Manual error: {0}")]
    Manual(String),
}
