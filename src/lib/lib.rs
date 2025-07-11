#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use futures::TryFutureExt;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    Git2Error(#[from] git2::Error),
}

enum Artefact {
    Tarball {
        size: u64,
        path: PathBuf,
    },
    Repository(git2::Repository),
}

impl std::fmt::Debug for Artefact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tarball{ size, path } => {
                let mut debug_struct = f.debug_struct("Tarball");
                debug_struct.field("size", size);
                debug_struct.field("path", path);
                debug_struct.finish()
            },
            Self::Repository(repo) => {
                let mut debug_struct = f.debug_struct("Repository");
                if let Some(workdir) = repo.workdir() {
                    debug_struct.field("workdir", &workdir.display().to_string());
                }
                match repo.head() {
                    Ok(head) => debug_struct.field("head", &head.name()),
                    Err(_) => debug_struct.field("head", &"<no head>"),
                };
                debug_struct.field("is_bare", &repo.is_bare());
                debug_struct.finish()
            }
        }
    }
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
    #[serde(default)]
    recursive: bool,
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

fn get_remote_sources_from_toml_table(table: &toml::Table) -> Result<Sources, serde_json::Error> {
    let sources: Sources = table
        .get("package").unwrap()
        .get("metadata").unwrap()
        .get("fetch-source").unwrap()
        .to_owned()
        .try_into()
        .unwrap();
    Ok(sources)
}

async fn fetch_source<'a>(name: &'a str, source: &'a Source, into_root: PathBuf) -> Result<(&'a str, Artefact), crate::Error> {
    let result = match source.get_url() {
        Url::Tar(url) => {
            fetch_tar_source(url, into_root.join(format!("{name}.tar.gz"))).await
        }
        Url::Git(url) => fetch_git_source(url, into_root.join(name))
    };
    result.map(|artefact| (name, artefact))
}

async fn fetch_tar_source(url: &str, path: PathBuf) -> Result<Artefact, crate::Error> {
    let mut f = fs::File::create(&path)?;
    println!("Fetching tarball: {url}");
    let response = reqwest::get(url).await?; // failed to fetch url
    let body = response.bytes().await?; // failed to read the response body
    let size = io::copy(&mut body.as_ref(), &mut f)?;
    Ok(Artefact::Tarball { size, path })
}

fn fetch_git_source<P>(url: &str, into: P) -> Result<Artefact, crate::Error>
where
    P: AsRef<Path>
{
    println!("Fetching git source from: {url}");
    let mut builder = git2::build::RepoBuilder::new();
    let mut fetch_options = git2::FetchOptions::new();
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(prepare_git_credentials);
    fetch_options.remote_callbacks(callbacks);
    builder.fetch_options(fetch_options);
    Ok(Artefact::Repository(builder.clone(url, into.as_ref())?))
}

fn prepare_git_credentials(
    url: &str,
    username_from_url: Option<&str>,
    credential_type: git2::CredentialType,
) -> Result<git2::Cred, git2::Error> {
    if credential_type.contains(git2::CredentialType::SSH_KEY) {
        let user = username_from_url.expect("The ssh link should include a username");
        let identity_file = std::env::var_os("GIT_IDENTITY_FILE")
            .map(PathBuf::from)
            .expect("GIT_IDENTITY_FILE environment variable should be set");
        git2::Cred::ssh_key(user, None, &identity_file, None)
    } else if credential_type.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
        let username = username_from_url.map(String::from).unwrap_or_else(|| {
            print!("Enter username for {url}: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        });
        let password =
            rpassword::prompt_password(format!("Enter password/PAT for {username}: ")).unwrap();
        git2::Cred::userpass_plaintext(&username, &password)
    } else {
        Err(git2::Error::from_str("Unsupported credential type, expected ssh or plaintext"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_sources_manually_extract() {
        let document = fs::read_to_string("Cargo.toml")
            .expect("Failed to read Cargo.toml")
            .parse::<toml::Table>().unwrap()
            ;
        let sources = get_remote_sources_from_toml_table(&document).unwrap();
        println!("{sources:#?}");        
    }

    #[test]
    fn test_fetch_sources_async() {
        let fetch_dir = PathBuf::from("test/test_fetch_sources_async");
        fs::create_dir_all(&fetch_dir).expect("Failed to create directory for fetching sources");
        let document = fs::read_to_string("Cargo.toml")
            .expect("Failed to read Cargo.toml")
            .parse::<toml::Table>().unwrap()
            ;
        let sources = get_remote_sources_from_toml_table(&document).unwrap();
        let tok = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        let results = tok.block_on(async {
            let futures = sources
                .iter()
                .map(|(n, s)| fetch_source(n, s, fetch_dir.clone()));
            futures::future::join_all(futures).await
        });
        println!("Fetched sources: {results:#?}");
    }
}
