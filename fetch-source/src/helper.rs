use std::path::Path;

use crate::source::{Artefact, Source, Sources};

pub(crate) mod serial {
    use super::*;

    pub fn fetch<S: AsRef<str>>(
        name: S,
        source: Source,
        out_dir: &std::path::Path,
    ) -> Result<Artefact, crate::Error> {
        source.fetch(name.as_ref(), out_dir)
    }

    pub fn fetch_all<S: AsRef<str>>(
        sources: Sources,
        out_dir: &Path,
    ) -> Vec<Result<Artefact, crate::Error>> {
        sources
            .into_iter()
            .map(|(n, s)| fetch(n, s, out_dir))
            .collect()
    }
}

#[cfg(feature = "rayon")]
pub(crate) mod parallel {
    use super::*;
    use rayon::prelude::*;

    pub fn fetch_all_par<S: AsRef<str>>(
        sources: Sources,
        out_dir: &Path,
    ) -> Vec<Result<Artefact, crate::Error>> {
        sources
            .into_par_iter()
            .map(|(n, s)| super::serial::fetch(n, s, out_dir))
            .collect::<Vec<_>>()
    }
}
