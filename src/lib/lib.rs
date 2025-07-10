#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::collections::HashMap;
use std::fs;
use std::io;

use futures::TryFutureExt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
}

enum Url<'a> {
    Tar(&'a str),
    Git(&'a str),
}

#[derive(Debug, serde::Deserialize)]
struct TarSource {
    #[serde(rename = "tar")]
    url: String,
}

#[derive(Debug, serde::Deserialize)]
enum GitReference {
    #[serde(rename = "branch")]
    Branch(String),
    #[serde(rename = "tag")]
    Tag(String),
    #[serde(rename = "rev")]
    Rev(String),
}

#[derive(Debug, serde::Deserialize)]
struct GitSource {
    #[serde(rename = "git")]
    url: String,
    #[serde(flatten)]
    reference: Option<GitReference>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum Source {
    Tar(TarSource),
    Git(GitSource),
}

impl Source {
    pub fn get_url(&self) -> Url<'_> {
        match self {
            Source::Tar(s) => Url::Tar(&s.url),
            Source::Git(s) => Url::Git(&s.url),
        }
    }
}

type Sources = HashMap<String, Source>;

fn get_remote_sources(metadata: &cargo_metadata::Metadata) -> Result<Sources, serde_json::Error> {
    if let Some(sources_table) = metadata
        .root_package()
        .expect("The root package should have a Cargo.toml")
        .metadata
        .as_object()
        .expect("The package.metadata value should be a table")
        .get("fetch-source")
    {
        serde_json::from_value(sources_table.clone())
    } else {
        todo!()
    }
}

async fn fetch_source(source: &Source) -> Result<(), crate::Error> {
    match source.get_url() {
        Url::Tar(url) => {
            // HACK!
            let dest = fs::File::create(url.split('/').next_back().unwrap())?;
            fetch_tar_source(url, dest).await
        }
        Url::Git(url) => fetch_git_source(url).await,
    }
}

async fn fetch_git_source(url: &str) -> Result<(), crate::Error> {
    println!("Fetching git source from: {url}");
    Ok(())
}

async fn fetch_tar_source(url: &str, mut dest: fs::File) -> Result<(), crate::Error> {
    println!("Fetching tarball: {url}");
    let response = reqwest::get(url).await?; // failed to fetch url
    let body = reqwest::get(url).and_then(|r| r.bytes()).await?; // failed to read the response body
    io::copy(&mut body.as_ref(), &mut dest)?; // failed to copy content
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_metadata::*;

    #[test]
    fn print_sources() {
        let meta = MetadataCommand::new().exec().unwrap();
        let sources = get_remote_sources(&meta).unwrap();
        println!("{sources:#?}");
    }

    #[test]
    fn test_fetch_sources_async() {
        let tok = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        let meta = MetadataCommand::new().exec().unwrap();
        let source_map = get_remote_sources(&meta).unwrap();
        let futures = source_map
            .values()
            .map(fetch_source)
            .collect::<Vec<_>>();
        let foo = tok.block_on(async { futures::future::join_all(futures).await });
        println!("Fetched sources: {foo:#?}");
    }

    // #[test]
    // fn test_metadata() {
    //     let meta = MetadataCommand::new().exec().unwrap();
    //     get_remote_sources(&meta);
    //     for (key, url) in sources {
    //         println!("==> fetch {key} from {url}");
    //     }
    // }

    // #[test]
    // fn test_download_tarball() {
    //     let meta = MetadataCommand::new().exec().unwrap();
    //     let sources = get_remote_sources(&meta).unwrap();
    //     for (key, url) in sources {
    //         println!("==> fetch {key} from {url}");
    //         let response = get(&url);
    //         let body = response
    //             .expect("Failed to fetch the source")
    //             .bytes()
    //             .expect("Failed to read the response body");
    //         let mut out = fs::File::create(format!("{key}.tar.gz")).expect("failed to create file");
    //         io::copy(&mut body.as_ref(), &mut out).expect("failed to copy content");

    //     }
    // }

    // #[tokio::test]
    // async fn test_download_tarball_async() {
    //     let meta = MetadataCommand::new().exec().unwrap();
    //     let sources = get_remote_sources(&meta).unwrap();
    //     for (key, url) in &sources {
    //         println!("==> async fetch {key} from {url}");
    //         let response = reqwest::get(url).await.unwrap_or_else(|_| panic!("Failed to fetch {key} from {url}"));
    //         let body = response
    //             .bytes()
    //             .await
    //             .expect("Failed to read the response body");
    //         let mut out = fs::File::create(format!("{key}.tar.gz")).expect("failed to create file");
    //         io::copy(&mut body.as_ref(), &mut out).expect("failed to copy content");
    //     }
    // }
}
