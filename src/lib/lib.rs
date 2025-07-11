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
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error("Manual error: {0}")]
    Manual(String),
}

enum Artefact {
    Tarball { size: u64, path: PathBuf },
    Repository(git2::Repository),
}

impl std::fmt::Debug for Artefact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tarball { size, path } => {
                let mut debug_struct = f.debug_struct("Tarball");
                debug_struct.field("size", size);
                debug_struct.field("path", path);
                debug_struct.finish()
            }
            Self::Repository(repo) => {
                let mut debug_struct = f.debug_struct("Repository");
                if let Some(workdir) = repo.workdir() {
                    debug_struct.field("workdir", &workdir.display().to_string());
                }
                match repo.head() {
                    Ok(head) => debug_struct.field("head", &head.target()),
                    Err(_) => debug_struct.field("head", &"<no head>"),
                };
                debug_struct.finish()
            }
        }
    }
}

#[derive(Debug)]
enum Url<'a> {
    Tar(&'a str),
    Git(&'a str),
}

impl std::fmt::Display for Url<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Url::Tar(url) => write!(f, "{url}"),
            Url::Git(url) => write!(f, "{url}"),
        }
    }
}

impl AsRef<str> for Url<'_> {
    fn as_ref(&self) -> &str {
        match self {
            Url::Tar(url) => url,
            Url::Git(url) => url,
        }
    }
}

trait GetUrl {
    fn url(&self) -> Url<'_>;
}

trait Fetch {
    fn fetch(&self, name: &str, dir: PathBuf) -> Result<Artefact, crate::Error>;
}

trait RepositoryExt {
    fn find_object_from_commit_sha(&self, commit_sha: &str) -> Result<git2::Object, git2::Error>;
    fn clone_into(url: &str, branch: Option<&str>, into: &Path) -> Result<git2::Repository, git2::Error>;
    fn checkout_commit(&self, sha: &str) -> Result<(), git2::Error>;
    fn update_submodules_recursive(&self) -> Result<(), git2::Error>;
    fn update_submodule(&self, submodule: &mut git2::Submodule) -> Result<(), git2::Error>;
    fn get_submodule_repository(&self, submodule: &git2::Submodule, top_level: PathBuf) -> Result<git2::Repository, git2::Error>;
}

impl RepositoryExt for git2::Repository {
    fn find_object_from_commit_sha(&self, sha: &str) -> Result<git2::Object, git2::Error> {
        if sha.len() < 40 {
            self.revparse_single(sha)
        } else {
            self.find_object_by_prefix(sha, Some(git2::ObjectType::Commit))
        }
    }

    fn clone_into(url: &str, branch: Option<&str>, into: &Path) -> Result<git2::Repository, git2::Error> {
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(prepare_git_credentials);
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);
        if let Some(branch) = branch {
            builder.branch(branch);
        }
        builder.clone(url, into)
    }
    
    fn checkout_commit(&self, sha: &str) -> Result<(), git2::Error> {
        let mut checkout_builder = git2::build::CheckoutBuilder::new();
        let commit = self.find_object_from_commit_sha(sha)?;
        self.set_head_detached(commit.id())?;
        self.checkout_tree(&commit, Some(&mut checkout_builder))
    }

    fn update_submodules_recursive(&self) -> Result<(), git2::Error> {
        for mut submodule in self.submodules()? {
            self.update_submodule(&mut submodule)?;
            if let Some(workdir) = self.workdir() {
                let repo = self.get_submodule_repository(&submodule, workdir.to_path_buf())?;
                repo.update_submodules_recursive()?;
            }
        }
        Ok(())
    }

    fn update_submodule(&self, submodule: &mut git2::Submodule) -> Result<(), git2::Error> {
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(prepare_git_credentials);
        let mut update_fetch_options = git2::FetchOptions::new();
        update_fetch_options.remote_callbacks(callbacks);
        let mut update_options = git2::SubmoduleUpdateOptions::new();
        update_options.fetch(update_fetch_options);
        submodule.update(true, Some(&mut update_options))
    }

    fn get_submodule_repository(&self, submodule: &git2::Submodule, top_level: PathBuf) -> Result<git2::Repository, git2::Error> {
        git2::Repository::open(top_level.join(submodule.path()))
    }
}

#[derive(Debug, serde::Deserialize)]
struct TarSource {
    #[serde(rename = "tar")]
    url: String,
}

impl TarSource {
    fn fetch_blocking_impl(url: &str, path: PathBuf) -> Result<Artefact, crate::Error> {
        let mut f = fs::File::create(&path)?;
        let response = reqwest::blocking::get(url)?;
        let body = response.bytes()?;
        let size = io::copy(&mut body.as_ref(), &mut f)?;
        Ok(Artefact::Tarball { size, path })
    }

    async fn fetch_async_impl(url: &str, path: PathBuf) -> Result<Artefact, crate::Error> {
        let mut f = fs::File::create(&path)?;
        let response = reqwest::get(url).await?;
        let body = response.bytes().await?;
        let size = io::copy(&mut body.as_ref(), &mut f)?;
        Ok(Artefact::Tarball { size, path })
    }

    async fn fetch_async(&self, dir: PathBuf) -> Result<Artefact, crate::Error> {
        let parsed = url::Url::parse(&self.url)?;
        let filename = parsed
            .path_segments()
            .and_then(|mut s| s.next_back())
            .expect("The URL should end in a filename");
        Self::fetch_async_impl(&self.url, dir.join(filename)).await
    }
}

impl GetUrl for TarSource {
    fn url(&self) -> Url<'_> {
        Url::Tar(&self.url)
    }
}

impl Fetch for TarSource {
    fn fetch(&self, _: &str, dir: PathBuf) -> Result<Artefact, crate::Error> {
        let parsed = url::Url::parse(&self.url)?;
        let filename = parsed
            .path_segments()
            .and_then(|mut s| s.next_back())
            .expect("The URL should end in a filename");
        Self::fetch_blocking_impl(&self.url, dir.join(filename))
    }
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

impl GitReference {
    fn as_branch_name(&self) -> Option<&str> {
        match self {
            GitReference::Branch(name) | GitReference::Tag(name) => {
                Some(name)
            }
            GitReference::Rev(_) => None
        }
    }
    fn as_commit_sha(&self) -> Option<&str> {
        match self {
            GitReference::Branch(name) | GitReference::Tag(name) => None,
            GitReference::Rev(commit_sha) => Some(commit_sha),
        }
    }
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

impl GitSource {
    pub fn is_recursive(&self) -> bool {
        self.recursive
    }

    fn branch_name(&self) -> Option<&str> {
        match self.reference.as_ref() {
            Some(GitReference::Branch(name)) | Some(GitReference::Tag(name)) => {
                Some(name)
            }
            _ => None
        }
    }

    fn commit_sha(&self) -> Option<&str> {
        match self.reference.as_ref() {
            Some(GitReference::Rev(commit_sha)) => Some(commit_sha),
            _ => None,
        }
    }
}

impl GetUrl for GitSource {
    fn url(&self) -> Url<'_> {
        Url::Git(&self.url)
    }
}

impl Fetch for GitSource {
    fn fetch(&self, name: &str, dir: PathBuf) -> Result<Artefact, crate::Error> {
        let repo = git2::Repository::clone_into(&self.url, self.branch_name(), &dir.join(name))?;
        if let Some(commit_sha) = self.commit_sha() {
            repo.checkout_commit(commit_sha)?;
        }
        if self.recursive {
            repo.update_submodules_recursive()?;
        }
        Ok(Artefact::Repository(repo))
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum Source {
    Tar(TarSource),
    Git(GitSource),
}

type Sources = HashMap<String, Source>;

fn get_remote_sources_from_toml_table(table: &toml::Table) -> Result<Sources, serde_json::Error> {
    let sources: Sources = table
        .get("package")
        .unwrap()
        .get("metadata")
        .unwrap()
        .get("fetch-source")
        .unwrap()
        .to_owned()
        .try_into()
        .unwrap();
    Ok(sources)
}

async fn fetch_source<'a>(
    name: &'a str,
    source: &'a Source,
    dir: PathBuf,
) -> Result<(&'a str, Artefact), crate::Error> {
    let result = match source {
        Source::Tar(tar) => tar.fetch_async(dir).await,
        Source::Git(git) => {
            if git.is_recursive() {
                println!("Fetching git source and all submodules from: {}", git.url());
            } else {
                println!("Fetching git source from: {}", git.url());
            }
            git.fetch(name, dir)
        }
    };
    result.map(|artefact| (name, artefact))
}

fn fetch_source_blocking<'a>(
    name: &'a str,
    source: &'a Source,
    dir: PathBuf,
) -> Result<(&'a str, Artefact), crate::Error> {
    let result = match source {
        Source::Tar(tar) => tar.fetch(name, dir),
        Source::Git(git) => {
            if git.is_recursive() {
                println!("Fetching git source and all submodules from: {}", git.url());
            } else {
                println!("Fetching git source from: {}", git.url());
            }
            git.fetch(name, dir)
        }
    };
    result.map(|artefact| (name, artefact))
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
        Err(git2::Error::from_str(
            "Unsupported credential type, expected ssh or plaintext",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_sources_manually_extract() {
        let document = fs::read_to_string("Cargo.toml")
            .expect("Failed to read Cargo.toml")
            .parse::<toml::Table>()
            .unwrap();
        let sources = get_remote_sources_from_toml_table(&document).unwrap();
        println!("{sources:#?}");
    }

    #[test]
    fn test_fetch_sources_async() {
        let fetch_dir = PathBuf::from("test/test_fetch_sources_async");
        fs::create_dir_all(&fetch_dir).expect("Failed to create directory for fetching sources");
        let document = fs::read_to_string("Cargo.toml")
            .expect("Failed to read Cargo.toml")
            .parse::<toml::Table>()
            .unwrap();
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

    #[test]
    fn test_fetch_sources_blocking() {
        let fetch_dir = PathBuf::from("test/test_fetch_sources_blocking");
        fs::create_dir_all(&fetch_dir).expect("Failed to create directory for fetching sources");
        let document = fs::read_to_string("Cargo.toml")
            .expect("Failed to read Cargo.toml")
            .parse::<toml::Table>()
            .unwrap();
        let sources = get_remote_sources_from_toml_table(&document).unwrap();
        let results = sources
            .iter()
            .map(|(n, s)| fetch_source_blocking(n, s, fetch_dir.clone()))
            .collect::<Vec<_>>();
        println!("Fetched sources: {results:#?}");
    }
}
