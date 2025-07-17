use std::path::PathBuf;

#[derive(Debug)]
pub enum Artefact {
    Tarball { items: Vec<std::path::PathBuf> },
    Repository(PathBuf),
}
