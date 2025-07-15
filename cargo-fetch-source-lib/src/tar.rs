use std::path::PathBuf;
use std::{fs, io};
use std::process::{Command, Stdio};

use crate::Fetch;
use crate::artefact::Artefact;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TarSource {
    #[serde(rename = "tar")]
    url: String,
}

impl TarSource {
    pub(crate) async fn fetch_async(&self, dir: PathBuf) -> Result<Artefact, crate::Error> {
        let parsed = url::Url::parse(&self.url)?;
        let filename = parsed
            .path_segments()
            .and_then(|mut s| s.next_back())
            .expect("The URL should end in a filename");
        Self::fetch_async_impl(&self.url, dir.join(filename)).await
    }

    fn fetch_blocking_impl(url: &str, path: PathBuf) -> Result<Artefact, crate::Error> {
        let mut f = fs::File::create(&path)?;
        let response = reqwest::blocking::get(url)?;
        let body = response.bytes()?;
        let size = io::copy(&mut body.as_ref(), &mut f)?;
        Ok(Artefact::Tarball { size, path })
    }

    async fn fetch_async_impl(url: &str, path: PathBuf) -> Result<Artefact, crate::Error> {
        let mut f = fs::File::create(&path)?;
        let response = reqwest::get(url).await?;
        let body = response.bytes().await?;
        let size = io::copy(&mut body.as_ref(), &mut f)?;
        Ok(Artefact::Tarball { size, path })
    }

    pub fn make_task<P: AsRef<std::path::Path>>(&self, root: P) -> Command {
    }
}

impl Fetch for TarSource {
    fn fetch(&self, _: &str, dir: PathBuf) -> Result<Artefact, crate::Error> {
        let parsed = url::Url::parse(&self.url)?;
        let filename = parsed
            .path_segments()
            .and_then(|mut s| s.next_back())
            .expect("The URL should end in a filename");
        Self::fetch_blocking_impl(&self.url, dir.join(filename))
    }
}
