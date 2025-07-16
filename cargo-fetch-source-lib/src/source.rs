use std::collections::HashMap;
use std::path::PathBuf;

use crate::artefact::Artefact;
use crate::git::GitSource;
use crate::tar::TarSource;

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum Source {
    Tar(TarSource),
    Git(GitSource),
}

impl Source {
    fn fetch(&self, name: &str, dir: PathBuf) -> Result<Artefact, crate::Error> {
        match self {
            Source::Tar(tar) => tar.fetch(name, dir),
            Source::Git(git) => git.fetch(name, dir),
        }
    }
}

pub(crate) type Sources = HashMap<String, Source>;

pub(crate) fn get_remote_sources_from_toml_table(
    table: &toml::Table,
) -> Result<Sources, serde_json::Error> {
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

pub(crate) fn fetch_source_blocking<'a>(
    name: &'a str,
    source: &'a Source,
    dir: PathBuf,
) -> Result<(&'a str, Artefact), crate::Error> {
    source.fetch(name, dir).map(|artefact| (name, artefact))
}
