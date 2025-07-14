use std::io::{Write, stdin, stdout};
use std::path::{Path, PathBuf};

pub(crate) use git2::{Error, Repository};

pub(crate) trait RepositoryExt {
    fn find_object_from_commit_sha(&self, commit_sha: &str) -> Result<git2::Object, git2::Error>;
    fn clone_into(
        url: &str,
        branch: Option<&str>,
        into: &Path,
    ) -> Result<git2::Repository, git2::Error>;
    fn checkout_commit(&self, sha: &str) -> Result<(), git2::Error>;
    fn update_submodules_recursive(&self) -> Result<(), git2::Error>;
}

trait SubmoduleExt {
    fn update_with<F>(&mut self, f: F) -> Result<(), git2::Error>
    where
        F: Fn(&str, Option<&str>, git2::CredentialType) -> Result<git2::Cred, git2::Error>;
    fn get_repository(&self, top_level: PathBuf) -> Result<git2::Repository, git2::Error>;
}

impl RepositoryExt for git2::Repository {
    fn find_object_from_commit_sha(&self, sha: &str) -> Result<git2::Object, git2::Error> {
        if sha.len() < 40 {
            self.revparse_single(sha)
        } else {
            self.find_object_by_prefix(sha, Some(git2::ObjectType::Commit))
        }
    }

    fn clone_into(
        url: &str,
        branch: Option<&str>,
        into: &Path,
    ) -> Result<git2::Repository, git2::Error> {
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
            submodule.update_with(prepare_git_credentials)?;
            if let Some(workdir) = self.workdir() {
                let repo = submodule.get_repository(workdir.to_path_buf())?;
                repo.update_submodules_recursive()?;
            }
        }
        Ok(())
    }
}

impl<'a> SubmoduleExt for git2::Submodule<'a> {
    fn update_with<F>(&mut self, fetch_credentials: F) -> Result<(), git2::Error>
    where
        F: Fn(&str, Option<&str>, git2::CredentialType) -> Result<git2::Cred, git2::Error>,
    {
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(fetch_credentials);
        let mut update_fetch_options = git2::FetchOptions::new();
        update_fetch_options.remote_callbacks(callbacks);
        let mut update_options = git2::SubmoduleUpdateOptions::new();
        update_options.fetch(update_fetch_options);
        self.update(true, Some(&mut update_options))
    }

    fn get_repository(&self, top_level: PathBuf) -> Result<git2::Repository, git2::Error> {
        git2::Repository::open(top_level.join(self.path()))
    }
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
            stdout().flush().unwrap();
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();
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
