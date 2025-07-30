// A BTree maintains key order
use std::collections::BTreeMap;

use crate::{Source, SourceArtefact};

const CACHE_FILE_NAME: &str = "fetch-source-cache.json";

#[derive(
    Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Clone,
)]
pub struct Digest(String);

impl AsRef<std::path::Path> for Digest {
    fn as_ref(&self) -> &std::path::Path {
        self.0.as_ref()
    }
}

/// The arguments required to fetch a missing source
pub struct NamedFetchSpec {
    /// The name this source is known by
    pub name: String,
    /// The source to be fetched
    pub source: Source,
    /// The destination for the fetched artefact
    pub path: std::path::PathBuf,
}

/// The cache is a collection of cached artefacts, indexed by their source's digest.
#[derive(Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Cache {
    #[serde(flatten)]
    map: BTreeMap<Digest, SourceArtefact>,
    #[serde(skip)]
    cache_file: std::path::PathBuf,
}

impl std::fmt::Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn __digest__<D: serde::Serialize>(data: &D) -> Digest {
    Digest(sha256::digest(serde_json::to_string(data).unwrap()))
}

impl Cache {
    fn create_at(cache_file: std::path::PathBuf) -> Self {
        Self {
            map: BTreeMap::new(),
            cache_file,
        }
    }

    /// Get the cache file path.
    pub fn cache_file(&self) -> &std::path::Path {
        &self.cache_file
    }

    /// Get the directory of the cache file
    pub fn cache_dir(&self) -> &std::path::Path {
        self.cache_file.parent().unwrap()
    }

    /// Calculate the path which a fetched source would have within the cache
    pub fn cached_path(&self, source: &Source) -> std::path::PathBuf {
        self.cache_dir().join(Self::digest(source))
    }

    /// Get the digest of a source
    fn digest(source: &Source) -> Digest {
        __digest__(source)
    }

    /// Partition a set of sources into those which are cached (giving their named digests) and
    /// those which are missing (giving their fetch specifications)
    pub fn partition_by_status<S>(&self, sources: S) -> (Vec<(String, Digest)>, Vec<NamedFetchSpec>)
    where
        S: Iterator<Item = (String, Source)>,
    {
        sources.fold(
            (Vec::new(), Vec::new()),
            |(mut cached, mut missing), (name, source)| {
                let digest = Self::digest(&source);
                if self.map.contains_key(&digest) {
                    cached.push((name, digest));
                } else {
                    let path = self.cached_path(&source);
                    missing.push(NamedFetchSpec { name, source, path });
                }
                (cached, missing)
            },
        )
    }

    /// Fetch and insert missing sources. Fetched sources are consumed and become cached artefacts.
    /// Return the digests of the cached source artefacts. Sources which couldn't be fetched are
    /// returned via errors.
    pub fn fetch_missing<F>(
        &mut self,
        sources: Vec<NamedFetchSpec>,
        fetch: F,
    ) -> (Vec<(String, Digest)>, Vec<crate::FetchError>)
    where
        F: FnOnce(Vec<NamedFetchSpec>) -> Vec<crate::FetchResult<(String, SourceArtefact)>>,
    {
        fetch(sources).into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut cached, mut errors), result| {
                match result {
                    Ok((name, artefact)) => cached.push((name, self.insert(artefact))),
                    Err(error) => errors.push(error),
                }
                (cached, errors)
            },
        )
    }

    /// Loads the cache from a JSON file in the given directory, creating a new cache if the file
    /// does not exist.
    pub fn load<P>(cache_dir: P) -> Result<Self, crate::Error>
    where
        P: AsRef<std::path::Path>,
    {
        let cache_file = cache_dir.as_ref().join(CACHE_FILE_NAME);
        if !cache_file.is_file() {
            Ok(Self::create_at(cache_file))
        } else {
            let cache: Self = serde_json::from_str(&std::fs::read_to_string(&cache_file)?)?;
            Ok(Self {
                map: cache.map,
                cache_file,
            })
        }
    }

    /// Saves the cache.
    pub fn save(&self) -> Result<(), crate::Error> {
        let json = serde_json::to_string_pretty(self)?;
        Ok(std::fs::write(&self.cache_file, json)?)
    }

    /// Check whether the cache file exists in the given directory.
    pub fn exists<P>(cache_dir: P) -> bool
    where
        P: AsRef<std::path::Path>,
    {
        cache_dir.as_ref().join(CACHE_FILE_NAME).is_file()
    }

    /// Cache a named source artefact and return its digest. Replaces the previous value for this
    /// source. Note that a source need not have a unique name.
    pub fn insert(&mut self, artefact: SourceArtefact) -> Digest {
        let digest = Self::digest(artefact.as_ref());
        self.map.insert(digest.clone(), artefact);
        digest
    }

    /// Check whether the cache contains the given source.
    pub fn is_cached(&self, source: &Source) -> bool {
        self.map.contains_key(&Self::digest(source))
    }

    /// Retrieves a cached value for the given source, if it exists.
    pub fn get<'a>(&'a self, source: &Source) -> Option<&'a SourceArtefact> {
        self.map.get(&Self::digest(source))
    }

    /// Get the artefact associated with a source's digest
    pub fn get_digest<'a>(&'a self, digest: &Digest) -> Option<&'a SourceArtefact> {
        self.map.get(digest)
    }

    /// Removes a cached value for the given source, returning it if it existed.
    pub fn remove(&mut self, source: &Source) -> Option<SourceArtefact> {
        self.map.remove(&Self::digest(source))
    }

    /// Returns an iterator over the cached values.
    pub fn values(&self) -> impl Iterator<Item = &SourceArtefact> {
        self.map.values()
    }

    /// Checks if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns the number of cached values.
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

impl IntoIterator for Cache {
    type Item = (Digest, SourceArtefact);
    type IntoIter = std::collections::btree_map::IntoIter<Digest, SourceArtefact>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a> IntoIterator for &'a Cache {
    type Item = (&'a Digest, &'a SourceArtefact);
    type IntoIter = std::collections::btree_map::Iter<'a, Digest, SourceArtefact>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! mock_cache_at {
        ($cache_file:expr) => {{
            Cache {
                map: BTreeMap::new(),
                cache_file: std::path::PathBuf::from($cache_file).join(CACHE_FILE_NAME),
            }
        }};
    }

    #[test]
    fn artefact_path_is_digest() {
        // The cache should determine the path to a cached artefact relative to the cache directory,
        // where the subdirectory is the digest of the source.
        let cache = mock_cache_at! {"/foo/bar"};
        let source: Source =
            crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
        assert_eq!(
            std::path::PathBuf::from("/foo/bar/").join(Cache::digest(&source)),
            cache.cached_path(&source)
        );
    }

    #[test]
    fn same_artefact_with_multiple_names_exists_once() {
        let mut cache = mock_cache_at! {"/foo/bar"};
        let artefact_1: crate::SourceArtefact = crate::build_from_json! {
            "tar": {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "artefact": "AAAAAAAA",
            }
        }
        .unwrap();
        let artefact_2: crate::SourceArtefact = crate::build_from_json! {
            "tar": {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "artefact": "BBBBBBBB",
            }
        }
        .unwrap();
        let digest_1 = cache.insert(artefact_1);
        let digest_2 = cache.insert(artefact_2);
        assert_eq!(cache.len(), 1);
        assert_eq!(digest_1, digest_2);
    }
}
