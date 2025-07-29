//! Support for declaring and fetching tar archives.

use super::error::FetchErrorInner;
use super::source::Artefact;

/// A definition of a tar archive to be (or which was) downloaded and extracted
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub struct TarSpec {
    #[serde(rename = "tar")]
    pub url: String,
}

/// Represents a remote tar archive to be downloaded and extracted.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub struct Tar {
    #[serde(flatten)]
    spec: TarSpec,
}

impl Tar {
    /// The upstream URL.
    pub fn upstream(&self) -> &str {
        &self.spec.url
    }

    /// Download and extract the tar archive directly into `dir`.
    pub fn fetch<P: AsRef<std::path::Path>>(&self, dir: P) -> Result<Artefact, FetchErrorInner> {
        let dir = dir.as_ref();
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
        let bytes = reqwest::blocking::get(self.spec.url.clone())?.bytes()?;
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_ref()));
        // Unpack the contents of the archive directly into the provided directory
        archive.unpack(dir)?;
        Ok(Artefact::Tar(TarArtefact {
            path: dir.to_path_buf(),
            remote: self.spec.clone(),
        }))
    }
}

impl std::fmt::Display for Tar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.spec.url)
    }
}

/// Represents a tar archive that has been downloaded and extracted.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub struct TarArtefact {
    pub path: std::path::PathBuf,
    pub remote: TarSpec,
}
