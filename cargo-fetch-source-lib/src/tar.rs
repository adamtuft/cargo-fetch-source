use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tar::Archive;
use flate2::read::GzDecoder;

use crate::Fetch;
use crate::artefact::Artefact;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TarSource {
    #[serde(rename = "tar")]
    url: String,
}

// impl TarSource {
//     pub(crate) async fn fetch_async(&self, dir: PathBuf) -> Result<Artefact, crate::Error> {
//         let parsed = url::Url::parse(&self.url)?;
//         let filename = parsed
//             .path_segments()
//             .and_then(|mut s| s.next_back())
//             .expect("The URL should end in a filename");
//         Self::fetch_async_impl(&self.url, dir.join(filename)).await
//     }

//     fn fetch_into_buf_blocking_impl<W: Write>(url: &str, mut buf: W) -> Result<u64, crate::Error> {
//         let bytes = reqwest::blocking::get(url)?.bytes()?;
//         Ok(io::copy(&mut bytes.as_ref(), &mut buf)?)
//     }

//     fn fetch_blocking_impl(url: &str, path: PathBuf) -> Result<Artefact, crate::Error> {
//         let mut f = fs::File::create(&path)?;
//         let size = Self::fetch_into_buf_blocking_impl(url, &mut f)?;
//         Ok(Artefact::Tarball { size, path })
//     }

//     async fn fetch_async_impl(url: &str, path: PathBuf) -> Result<Artefact, crate::Error> {
//         let mut f = fs::File::create(&path)?;
//         let response = reqwest::get(url).await?;
//         let body = response.bytes().await?;
//         let size = io::copy(&mut body.as_ref(), &mut f)?;
//         Ok(Artefact::Tarball { size, path })
//     }
// }

fn extract_tar_from_bytes(compressed_archive: &[u8], into: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let mut extracted_files: Vec<PathBuf> = Vec::new();
    let mut decompressed_archive = Vec::new();
    let mut decoder = GzDecoder::new(compressed_archive);
    decoder.read_to_end(&mut decompressed_archive)?;
    for archive_entry in Archive::new(std::io::Cursor::new(decompressed_archive)).entries()? {
        let mut archive_entry = archive_entry?;
        let header = archive_entry.header();
        let relative_path = header.path()?;
        let mut path = into.to_path_buf();
        path.push(&relative_path);
        if header.entry_type().is_dir() {
            std::fs::create_dir_all(&path)?;
        } else {
            if let Some(p) = path.parent() && !p.exists() {
                std::fs::create_dir_all(p)?;
            }
            let mut file_buffer = Vec::new();
            archive_entry.read_to_end(&mut file_buffer)?;
            let mut fs_file = fs::File::create(&path)?;
            fs_file.write_all(&file_buffer)?;
            extracted_files.push(path);
        }
    }
    Ok(extracted_files)
}

fn extract_tar(tar_file: &PathBuf, into: &Path) -> Result<Vec<PathBuf>, io::Error> {
    extract_tar_from_bytes(&fs::read(tar_file)?, into)
}

impl Fetch for TarSource {
    fn fetch(&self, _: &str, dir: PathBuf) -> Result<Artefact, crate::Error> {
        let mut compressed_archive: Vec<u8> = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut compressed_archive);
        let payload = reqwest::blocking::get(&self.url)?.bytes()?;
        io::copy(&mut payload.as_ref(), &mut cursor)?;
        Ok(extract_tar_from_bytes(&compressed_archive, &dir).map(|items| Artefact::Tarball { items })?)
    }
}

#[cfg(test)]
mod test_flate2_decode {
    use super::*;
    use std::io::prelude::*;
    use std::io;
    use flate2::read::GzDecoder;
    use std::fs::File;
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
                let root = path.components().next().unwrap()
                    .as_os_str().to_string_lossy().to_string();
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
        let archive = PathBuf::from("/home/adam/git/cargo-fetch-source/cargo-fetch-source-lib/test/test_fetch_sources_blocking/otf2-2.3.tar.gz");
        let dest = PathBuf::from("/home/adam/git/cargo-fetch-source/cargo-fetch-source-lib/x");
        let result = super::extract_tar(&archive, &dest);
        println!("Extracted directories: {result:#?}");
    }
}
