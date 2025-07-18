//! Support for declaring and fetching tar archives.

use flate2::read::GzDecoder;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::{fs, io};
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
    pub items: TarItems
}

/// Represents a remote tar archive.
#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct Tar {
    #[serde(rename = "tar")]
    url: String,
}

impl Tar {
    /// Download and extract the archive into `dir`.
    pub fn fetch<P: AsRef<std::path::Path>>(self, _: &str, dir: P) -> Result<Artefact, Error> {
        let bytes = reqwest::blocking::get(&self.url)?.bytes()?;
        let archive = decompress(&bytes)?;
        let items = extract_tar_archive(&archive, dir.as_ref())?;
        Ok(Artefact::Tar(TarArtefact{ url: self.url, items }))
    }

    /// Download and extract the archive into `dir`. Consumes inputs to move data into the async
    /// context. Requires `async` feature.
    #[cfg(feature = "async")]
    pub async fn fetch_async(self, _: &str, dir: PathBuf) -> Result<Artefact, Error> {
        let bytes = reqwest::get(&self.url).await?.bytes().await?;
        let archive = decompress(&bytes)?;
        let items = extract_tar_archive(&archive, &dir)?;
        Ok(Artefact::Tar(TarArtefact{ url: self.url, items }))
    }

    /// The remote URL.
    pub fn upstream(&self) -> &str {
        &self.url
    }
}

fn decompress<Data: AsRef<[u8]>>(input: Data) -> Result<Vec<u8>, io::Error> {
    let mut decoder = GzDecoder::new(input.as_ref());
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

/// Extract the first path component from the rest. Panics if `comps` doesn't have at least one
/// component, because we expect to only receive valid paths from a tar archive.
fn split_first_component(mut comps: std::path::Components<'_>) -> (PathBuf, PathBuf) {
    let first = comps
        .next()
        .unwrap_or_else(|| panic!("There should be at least one path component"));
    let first = PathBuf::from(first.as_os_str().to_string_lossy().to_string());
    (first, comps.collect::<PathBuf>())
}

/// Extract files from a tar archive in memory. Return the extracted items.
fn extract_tar_archive(archive: &[u8], out_dir: &Path) -> Result<TarItems, io::Error> {
    let mut extracted_files: TarItems = Default::default();
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
            if let Some(p) = dest.parent()
                && !p.exists()
            {
                std::fs::create_dir_all(p)?;
            }
            let (first, rest) = split_first_component(path_in_archive.components());
            // Note the path to the extracted file
            if !extracted_files.contains_key(&first) {
                extracted_files.insert(first.clone(), Vec::new());
            }
            // SAFETY: can unwrap here as we just ensured this key exists.
            let items = extracted_files.get_mut(&first).unwrap();
            items.push(rest);
            let mut out_file = fs::File::create(&dest)?;
            std::io::copy(&mut archive_entry, &mut out_file)?;
        }
    }
    Ok(extracted_files)
}
