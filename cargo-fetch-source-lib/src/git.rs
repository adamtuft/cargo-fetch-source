use crate::Fetch;
use crate::artefact::Artefact;
use crate::git2_ext::{Repository, RepositoryExt};

#[derive(Debug, serde::Deserialize)]
pub(crate) enum GitReference {
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
            GitReference::Branch(name) | GitReference::Tag(name) => Some(name),
            GitReference::Rev(_) => None,
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
pub(crate) struct GitSource {
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

    fn branch_name(&self) -> Option<&str> {
        match self.reference.as_ref() {
            Some(GitReference::Branch(name)) | Some(GitReference::Tag(name)) => Some(name),
            _ => None,
        }
    }

    fn commit_sha(&self) -> Option<&str> {
        match self.reference.as_ref() {
            Some(GitReference::Rev(commit_sha)) => Some(commit_sha),
            _ => None,
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

impl Fetch for GitSource {
    fn fetch(&self, name: &str, dir: std::path::PathBuf) -> Result<Artefact, crate::Error> {
        let repo = Repository::clone_into(&self.url, self.branch_name(), &dir.join(name))?;
        if let Some(commit_sha) = self.commit_sha() {
            repo.checkout_commit(commit_sha)?;
        }
        if self.recursive {
            repo.update_submodules_recursive()?;
        }
        Ok(Artefact::Repository(repo))
    }
}
