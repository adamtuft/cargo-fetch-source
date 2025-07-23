//! Support for declaring and fetching tar archives.

use flate2::read::GzDecoder;
use std::io::prelude::*;
use std::path::Path;
use tar::Archive;

use super::error::Error;
use super::source::Artefact;

/// A map of items extracted from an archive. Keys are either top-level files, or top-level
/// directies mapped to a list of their contents.
pub type TarItems = std::collections::HashMap<std::path::PathBuf, Vec<std::path::PathBuf>>;

/// Represents the items extracted from a tar archive.
#[derive(Debug)]
pub struct TarArtefact {
    pub url: String,
    pub path: std::path::PathBuf,
}

/// Represents a remote tar archive.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Tar {
    #[serde(rename = "tar")]
    url: String,
}

impl Tar {
    fn extract<P: AsRef<std::path::Path>>(
        self,
        bytes: bytes::Bytes,
        name: &str,
        dir: P,
    ) -> Result<Artefact, Error> {
        let archive = decompress(&bytes)?;
        let sub_path = std::path::PathBuf::from_iter(name.split("::"));
        let dir = dir.as_ref().join(&sub_path);
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        extract_tar_archive(&archive, &dir)?;
        Ok(Artefact::Tar(TarArtefact {
            url: self.url,
            path: dir,
        }))
    }

    /// Download and extract the archive into `dir`.
    pub fn fetch<P: AsRef<std::path::Path>>(self, name: &str, dir: P) -> Result<Artefact, Error> {
        let bytes = reqwest::blocking::get(&self.url)?.bytes()?;
        self.extract(bytes, name, dir)
    }

    /// Download and extract the archive into `dir`. Consumes inputs to move data into the async
    /// context. Requires `async` feature.
    #[cfg(feature = "async")]
    pub async fn fetch_async(self, _: &str, dir: PathBuf) -> Result<Artefact, Error> {
        let bytes = reqwest::get(&self.url).await?.bytes().await?;
        self.extract(bytes, name, dir)
    }

    /// The remote URL.
    pub fn upstream(&self) -> &str {
        &self.url
    }
}

fn decompress<Data: AsRef<[u8]>>(input: Data) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = GzDecoder::new(input.as_ref());
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

/// Extract files from a tar archive in memory. Return the extracted items.
fn extract_tar_archive(archive: &[u8], out_dir: &Path) -> Result<(), std::io::Error> {
    for archive_entry in Archive::new(archive).entries()? {
        let mut archive_entry = archive_entry?;
        let header = archive_entry.header();
        let path_in_archive = header.path()?;
        // Construct the actual destination path relative to the output directory
        let mut dest = out_dir.to_path_buf();
        dest.push(&path_in_archive);
        if header.entry_type().is_dir() {
            std::fs::create_dir_all(&dest)?;
        } else {
            if let Some(name) = dest.iter().next_back()
                && name == "pax_global_header"
            {
                continue;
            }
            if let Some(p) = dest.parent()
                && !p.exists()
            {
                std::fs::create_dir_all(p)?;
            }
            let mut out_file = std::fs::File::create(&dest)?;
            std::io::copy(&mut archive_entry, &mut out_file)?;
        }
    }
    Ok(())
}
