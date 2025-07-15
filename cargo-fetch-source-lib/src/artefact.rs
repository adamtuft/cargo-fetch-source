use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum Artefact {
    Tarball { items: Vec<std::path::PathBuf> },
    Repository(PathBuf),
}
