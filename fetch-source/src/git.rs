//! Support for declaring and fetching git repositories.

use std::io::Read;

use super::error::Error;
use super::source::Artefact;

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub enum GitReference {
    #[serde(rename = "branch")]
    Branch(String),
    #[serde(rename = "tag")]
    Tag(String),
    #[serde(rename = "rev")]
    Rev(String),
}

/// A definition of a git remote to be (or which was) cloned
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub struct GitSpec {
    #[serde(rename = "git")]
    pub url: String,
    #[serde(flatten)]
    pub reference: Option<GitReference>,
    #[serde(default)]
    pub recursive: bool,
}

/// Represents a git repo cloned according to a source definition
#[derive(Debug)]
pub struct GitArtefact {
    pub local: std::path::PathBuf,
    pub remote: GitSpec,
}

/// Represents a remote git repository to be cloned.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub struct Git {
    #[serde(flatten)]
    spec: GitSpec,
}

impl Git {
    /// The upstream URL.
    pub fn upstream(&self) -> &str {
        &self.spec.url
    }

    /// Whether this repo will be cloned recursively.
    pub fn is_recursive(&self) -> bool {
        self.spec.recursive
    }

    /// The selected branch or tag name, if any.
    pub fn branch_name(&self) -> Option<&str> {
        match self.spec.reference.as_ref() {
            Some(GitReference::Branch(name)) | Some(GitReference::Tag(name)) => Some(name),
            _ => None,
        }
    }

    /// The selected commit SHA, if any.
    pub fn commit_sha(&self) -> Option<&str> {
        match self.spec.reference.as_ref() {
            Some(GitReference::Rev(commit_sha)) => Some(commit_sha),
            _ => None,
        }
    }

    /// Clone the repository into `dir`.
    pub fn fetch<P: AsRef<std::path::Path>>(&self, name: &str, dir: P) -> Result<Artefact, Error> {
        let sub_path = std::path::PathBuf::from_iter(name.split("::"));
        let local = dir.as_ref().join(&sub_path);
        if !local.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        let mut proc = self.clone_repo_subprocess(&local).spawn()?;
        let status = proc.wait()?;
        if status.success() {
            Ok(Artefact::Git(GitArtefact {
                local,
                remote: self.spec.clone(),
            }))
        } else {
            let mut stderr = String::new();
            if let Some(mut stderr_pipe) = proc.stderr.take() {
                stderr_pipe.read_to_string(&mut stderr)?;
            }
            let mut command = "git clone ".to_string();
            if let Some(branch) = self.branch_name() {
                command.push_str(&format!("--branch {branch} "));
            } else if let Some(commit_sha) = self.commit_sha() {
                command.push_str(&format!("--revision {commit_sha} "));
            }
            if self.spec.recursive {
                command.push_str("--recurse-submodules --shallow-submodules");
            }
            command.push_str(&format!("{} {}", self.spec.url, local.display()));
            let root_cause = anyhow::anyhow!(stderr);
            Err(Error::subprocess(command, status, root_cause))
        }
    }

    fn clone_repo_subprocess<P: AsRef<std::path::Path>>(&self, into: P) -> std::process::Command {
        let mut git = std::process::Command::new("git");
        git.args(["clone", "--depth", "1", "--no-tags"]);
        if let Some(branch) = self.branch_name() {
            git.args(["--branch", branch]);
        } else if let Some(commit_sha) = self.commit_sha() {
            git.args(["--revision", commit_sha]);
        }
        if self.spec.recursive {
            git.args(["--recurse-submodules", "--shallow-submodules"]);
        }
        git.arg(&self.spec.url).arg(into.as_ref());
        git.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());
        git
    }
}

impl std::fmt::Display for Git {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.spec.url)?;
        if let Some(reference) = &self.spec.reference {
            match reference {
                GitReference::Branch(branch) => write!(f, " (branch: {branch})")?,
                GitReference::Tag(tag) => write!(f, " (tag: {tag})")?,
                GitReference::Rev(rev) => write!(f, " (rev: {rev})")?,
            }
        }
        if self.spec.recursive {
            write!(f, " [recursive]")?;
        }
        Ok(())
    }
}
