//! Support for declaring and fetching git repositories.

use std::io::Read;

use super::error::FetchErrorInner;

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub enum GitReference {
    #[serde(rename = "branch")]
    Branch(String),
    #[serde(rename = "tag")]
    Tag(String),
    #[serde(rename = "rev")]
    Rev(String),
}

/// Represents a remote git repository to be cloned.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
pub struct Git {
    #[serde(rename = "git")]
    url: String,
    #[serde(flatten)]
    reference: Option<GitReference>,
    #[serde(default)]
    recursive: bool,
}

impl Git {
    /// The upstream URL.
    pub fn upstream(&self) -> &str {
        &self.url
    }

    /// Whether this repo will be cloned recursively.
    pub fn is_recursive(&self) -> bool {
        self.recursive
    }

    /// The selected branch or tag name, if any.
    pub fn branch_name(&self) -> Option<&str> {
        match self.reference.as_ref() {
            Some(GitReference::Branch(name)) | Some(GitReference::Tag(name)) => Some(name),
            _ => None,
        }
    }

    /// The selected commit SHA, if any.
    pub fn commit_sha(&self) -> Option<&str> {
        match self.reference.as_ref() {
            Some(GitReference::Rev(commit_sha)) => Some(commit_sha),
            _ => None,
        }
    }

    /// Clone the repository into `dir`.
    pub(crate) fn fetch<P: AsRef<std::path::Path>>(
        &self,
        dir: P,
    ) -> Result<std::path::PathBuf, FetchErrorInner> {
        if !dir.as_ref().exists() {
            std::fs::create_dir_all(&dir)?;
        }
        let mut proc = self.clone_repo_subprocess(dir.as_ref()).spawn()?;
        let status = proc.wait()?;
        let full_path = dir.as_ref().to_path_buf();
        if status.success() {
            Ok(full_path)
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
            if self.recursive {
                command.push_str("--recurse-submodules --shallow-submodules");
            }
            command.push_str(&format!("{} {}", self.url, full_path.display()));
            let root_cause = anyhow::anyhow!(stderr);
            Err(FetchErrorInner::subprocess(command, status, root_cause))
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
        if self.recursive {
            git.args(["--recurse-submodules", "--shallow-submodules"]);
        }
        git.arg(&self.url).arg(into.as_ref());
        git.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());
        git
    }
}

impl std::fmt::Display for Git {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.url)?;
        if let Some(reference) = &self.reference {
            match reference {
                GitReference::Branch(branch) => write!(f, " (branch: {branch})")?,
                GitReference::Tag(tag) => write!(f, " (tag: {tag})")?,
                GitReference::Rev(rev) => write!(f, " (rev: {rev})")?,
            }
        }
        if self.recursive {
            write!(f, " [recursive]")?;
        }
        Ok(())
    }
}
