use std::path::PathBuf;

pub(crate) enum Artefact {
    Tarball { size: u64, path: PathBuf },
    Repository(crate::git2_ext::Repository),
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
