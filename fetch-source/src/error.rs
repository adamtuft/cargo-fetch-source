/// The main error enum for this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
    #[error("Command '{command}' exited with status {status}\n{stderr}")]
    Subprocess {
        command: String,
        status: std::process::ExitStatus,
        stderr: String,
    },
}
