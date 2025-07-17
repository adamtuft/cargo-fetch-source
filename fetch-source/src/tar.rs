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

/// Represents a remote tar archive.
#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct Tar {
    #[serde(rename = "tar")]
    url: String,
}

impl Tar {
    /// Download and extract the archive into `dir`.
    pub fn fetch<P: AsRef<std::path::Path>>(&self, _: &str, dir: P) -> Result<Artefact, Error> {
        let mut compressed_archive: Vec<u8> = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut compressed_archive);
        let payload = reqwest::blocking::get(&self.url)?.bytes()?;
        io::copy(&mut payload.as_ref(), &mut cursor)?;
        Ok(extract_tar_from_bytes(&compressed_archive, dir.as_ref())
            .map(|items| Artefact::Tarball { items })?)
    }

    /// The remote URL.
    pub fn upstream(&self) -> &str {
        &self.url
    }
}

fn write_entry_to_file<'a, P, R>(entry: &mut tar::Entry<'a, R>, path: P) -> Result<(), io::Error>
where
    P: AsRef<Path>,
    R: std::io::Read + 'a,
{
    let mut out_buf = Vec::new();
    entry.read_to_end(&mut out_buf)?;
    let mut out_file = fs::File::create(path.as_ref())?;
    out_file.write_all(&out_buf)
}

fn decompress_archive(compressed: &[u8]) -> Result<Vec<u8>, io::Error> {
    let mut decoder = GzDecoder::new(compressed);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

fn read_archive_data<T>(data: T) -> Archive<std::io::Cursor<T>>
where
    std::io::Cursor<T>: std::io::Read,
{
    Archive::new(std::io::Cursor::new(data))
}

/// Extract the first path component from the rest. Panics if `comps` doesn't have at least one
/// component.
fn split_first_component(mut comps: std::path::Components<'_>) -> (PathBuf, PathBuf) {
    let first = comps
        .next()
        .unwrap_or_else(|| panic!("There should be at least one path component"));
    let first = PathBuf::from(first.as_os_str().to_string_lossy().to_string());
    (first, comps.collect::<PathBuf>())
}

/// Extract files from a compressed tar archive in memory. Return the extracted items.
fn extract_tar_from_bytes(compressed: &[u8], out_dir: &Path) -> Result<TarItems, io::Error> {
    let mut extracted_files: TarItems = Default::default();
    let archive = decompress_archive(compressed)?;
    for archive_entry in read_archive_data(archive).entries()? {
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
            write_entry_to_file(&mut archive_entry, &dest)?;
        }
    }
    Ok(extracted_files)
}

fn extract_tar(tar_file: &PathBuf, into: &Path) -> Result<TarItems, io::Error> {
    extract_tar_from_bytes(&fs::read(tar_file)?, into)
}

#[cfg(test)]
mod test_flate2_decode {
    use super::*;
    use flate2::read::GzDecoder;
    use std::fs::File;
    use std::io;
    use tar::Archive;

    // Uncompresses a Gz Encoded vector of bytes and returns a string or error
    // Here &[u8] implements Read
    fn decode_reader(bytes: Vec<u8>) -> io::Result<Vec<u8>> {
        let mut gz = GzDecoder::new(&bytes[..]);
        let mut buf = Vec::new();
        gz.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn extract_tar(bytes: &[u8]) -> Vec<String> {
        let mut a = Archive::new(bytes);
        let mut root_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();
        for file in a.entries().unwrap() {
            let mut file = file.unwrap();
            let path = file.header().path().unwrap();
            std::fs::create_dir_all(path.parent().unwrap()).expect("Failed to create directory");
            if file.header().entry_type().is_dir() {
                // println!("create {}", path.display());
                fs::create_dir(path.clone()).expect("Failed to create directory");
                // Print the first component of the path
                let root = path
                    .components()
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_string_lossy()
                    .to_string();
                // println!("Directory: {root}");
                root_dirs.insert(root);
            } else {
                // Write the file to disk
                let mut f = File::create(path).expect("Failed to create file");
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).expect("Failed to read file");
                f.write_all(buf.as_slice()).expect("Failed to write file");
            }
        }
        println!("Root directories: {root_dirs:?}");
        root_dirs.into_iter().collect()
    }

    #[test]
    fn test_decode_reader_1() {
        let file = fs::read("/home/adam/git/cargo-fetch-source/cargo-fetch-source-lib/test/test_fetch_sources_blocking/otf2-2.3.tar.gz").expect("Failed to read test file");
        let decoded = decode_reader(file).expect("Failed to decode gzipped file");
        println!("Decoded content: {} bytes", decoded.len());
        extract_tar(&decoded);
    }

    #[test]
    fn test_decode_reader_2() {
        let archive = PathBuf::from(
            "/home/adam/git/cargo-fetch-source/cargo-fetch-source-lib/test/test_fetch_sources_blocking/otf2-2.3.tar.gz",
        );
        let dest = PathBuf::from("/home/adam/git/cargo-fetch-source/cargo-fetch-source-lib/x");
        let result = super::extract_tar(&archive, &dest);
        println!("Extracted directories: {result:#?}");
    }
}
