//! Support for declaring and fetching tar archives.

use super::error::FetchErrorInner;

/// Represents a remote tar archive to be downloaded and extracted.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub struct Tar {
    #[serde(rename = "tar")]
    pub(crate) url: String,
}

impl Tar {
    /// The upstream URL.
    pub fn upstream(&self) -> &str {
        &self.url
    }

    /// Download and extract the tar archive directly into `dir`.
    pub(crate) fn fetch<P: AsRef<std::path::Path>>(
        &self,
        dir: P,
    ) -> Result<std::path::PathBuf, FetchErrorInner> {
        let dir = dir.as_ref();
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
        let bytes = reqwest::blocking::get(self.url.clone())?.bytes()?;
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_ref()));
        // Unpack the contents of the archive directly into the provided directory
        archive.unpack(dir)?;
        Ok(dir.to_path_buf())
    }
}

impl std::fmt::Display for Tar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.url)
    }
}
