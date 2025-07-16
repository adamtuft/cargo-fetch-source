

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
    #[error("Command '{command}' exited with status {status}\n{stderr}")]
    Subprocess {
        command: String,
        status: std::process::ExitStatus,
        stderr: String,
    },
}
