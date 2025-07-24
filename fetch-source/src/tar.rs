//! Support for declaring and fetching tar archives.

use flate2::read::GzDecoder;
use std::io::prelude::*;
use std::path::Path;
use tar::Archive;

use super::error::Error;
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

    /// Download and extract the tar archive into `dir`.
    pub fn fetch<P: AsRef<std::path::Path>>(&self, name: &str, dir: P) -> Result<Artefact, Error> {
        let sub_path = std::path::PathBuf::from_iter(name.split("::"));
        let local = dir.as_ref().join(&sub_path);
        if !local.exists() {
            std::fs::create_dir_all(&local)?;
        }
        self.download_and_extract(name, &local)?;
        Ok(Artefact::Tar(TarArtefact {
            path: local,
            remote: self.spec.clone(),
        }))
    }

    #[cfg(not(feature = "async"))]
    fn download_and_extract<P: AsRef<std::path::Path>>(
        &self,
        name: &str,
        dir: P,
    ) -> Result<(), Error> {
        let bytes = reqwest::blocking::get(self.spec.url.clone())?.bytes()?;
        self.extract(bytes, name, dir.as_ref())
    }

    /// Download and extract the archive into `dir`. Consumes inputs to move data into the async
    /// context. Requires `async` feature.
    #[cfg(feature = "async")]
    async fn download_and_extract<P: AsRef<std::path::Path>>(
        &self,
        name: &str,
        dir: P,
    ) -> Result<(), Error> {
        let bytes = reqwest::get(self.spec.url.clone()).await?.bytes().await?;
        self.extract(bytes, name, dir.as_ref())
    }

    fn extract<P: AsRef<std::path::Path>>(
        &self,
        bytes: bytes::Bytes,
        name: &str,
        dir: P,
    ) -> Result<(), Error> {
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_ref()));
        // Unpack the contents of the archive directly into the provided directory
        archive.unpack(dir.as_ref())?;
        Ok(())
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

/// Represents a tar archive to be downloaded and extracted.
pub type TarItems = std::collections::HashMap<std::path::PathBuf, Vec<std::path::PathBuf>>;
