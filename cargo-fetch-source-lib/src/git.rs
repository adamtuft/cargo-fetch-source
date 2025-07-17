use std::io::Read;

use crate::Artefact;

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub(crate) enum GitReference {
    #[serde(rename = "branch")]
    Branch(String),
    #[serde(rename = "tag")]
    Tag(String),
    #[serde(rename = "rev")]
    Rev(String),
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct GitSource {
    #[serde(rename = "git")]
    pub(crate) url: String,
    #[serde(flatten)]
    pub(crate) reference: Option<GitReference>,
    #[serde(default)]
    pub(crate) recursive: bool,
}

impl GitSource {
    pub fn is_recursive(&self) -> bool {
        self.recursive
    }

    pub fn branch_name(&self) -> Option<&str> {
        match self.reference.as_ref() {
            Some(GitReference::Branch(name)) | Some(GitReference::Tag(name)) => Some(name),
            _ => None,
        }
    }

    pub fn commit_sha(&self) -> Option<&str> {
        match self.reference.as_ref() {
            Some(GitReference::Rev(commit_sha)) => Some(commit_sha),
            _ => None,
        }
    }

    pub fn fetch(&self, name: &str, dir: std::path::PathBuf) -> Result<Artefact, crate::Error> {
        let repo = dir.join(name);
        let mut proc = crate::process::git_clone_task(self, &repo).spawn()?;
        let status = proc.wait()?;
        if status.success() {
            Ok(Artefact::Repository(repo))
        } else {
            let mut stderr = String::new();
            if let Some(mut stderr_pipe) = proc.stderr.take() {
                stderr_pipe.read_to_string(&mut stderr)?;
            }
            let command = format!("git clone {self}");
            Err(crate::Error::Subprocess {
                status,
                command,
                stderr,
            })
        }
    }
}

impl std::fmt::Display for GitSource {
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
