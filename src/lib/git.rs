use crate::Fetch;
use crate::artefact::Artefact;
use crate::git2_ext::{Repository, RepositoryExt};

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

#[cfg(test)]
mod test {
    use super::*;
    use std::process as proc;

    fn git_clone(url: &str, name: &str, reference: Option<GitReference>, recursive: bool) -> proc::Command {
        let mut git = proc::Command::new("git");
        git.arg("clone");
        match &reference {
            Some(GitReference::Branch(s)) | Some(GitReference::Tag(s)) => {
                git.arg("-b").arg(s);
            }
            Some(GitReference::Rev(s)) => {
                git.arg("--revision").arg(s);
            }
            None => {}
        }
        if recursive {
            git.arg("--recurse-submodules");
        }
        git.arg(url)
            .arg(format!("test/test_git_clone_subprocess/{name}"));
        git
    }
            
    
    #[test]
    fn test_git_clone_subprocess() {
        let document = std::fs::read_to_string("Cargo.toml")
            .expect("Failed to read Cargo.toml")
            .parse::<toml::Table>()
            .unwrap();
        let sources = crate::source::get_remote_sources_from_toml_table(&document).unwrap();
        for (name, source) in &sources {
            
        }
        let url = "git@github.com:adamtuft/dotfiles.git";
        let r = git_clone(url, "dotfiles", None, false).status().expect("Failed to execute git clone");
        println!("{r:?}");
        let r = git_clone(url, "dotfiles-cosma", Some(GitReference::Branch("cosma".to_string())), false).status().expect("Failed to execute git clone");
        println!("{r:?}");
        let r = git_clone(url, "dotfiles-fafd64a", Some(GitReference::Rev("82fbfb0fe2b037676610710069325f703613935f".to_string())), true).status().expect("Failed to execute git clone");
        println!("{r:?}");
    }
}
