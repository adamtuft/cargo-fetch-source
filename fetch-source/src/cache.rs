// A BTree maintains key order
use std::collections::BTreeMap;

use crate::{Source, SourceArtefact, Sources};

const CACHE_FILE_NAME: &str = "fetch-source-cache.json";

pub type CachedSources<K> = BTreeMap<K, MaybeCached<Source>>;

/// Represents whether a given Source is cached or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachedState {
    Cached,
    NotCached,
}

pub type MaybeCached<V> = (CachedState, V);

impl CachedState {
    pub fn cached<V>(value: V) -> MaybeCached<V> {
        (CachedState::Cached, value)
    }

    pub fn not_cached<V>(value: V) -> MaybeCached<V> {
        (CachedState::NotCached, value)
    }
}


/// The cache is a collection of cached artefacts, indexed by their source's digest.
#[derive(Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Cache {
    #[serde(flatten)]
    map: BTreeMap<String, SourceArtefact>,
    #[serde(skip)]
    cache_file: std::path::PathBuf,
}

fn __digest__<D: serde::Serialize>(data: &D) -> String {
    sha256::digest(serde_json::to_string(data).unwrap())
}

impl Cache {
    fn create_at(cache_file: std::path::PathBuf) -> Self {
        Self {
            map: BTreeMap::new(),
            cache_file,
        }
    }

    /// Tag sources as cached or not cached
    pub fn check_cached<S, K>(&self, sources: S) -> CachedSources<K>
    where
        S: IntoIterator<Item = (K, Source)>,
        K: Ord,
    {
        sources
            .into_iter()
            .map(|(key, source)| (key, self.into_maybe_cached(source)))
            .collect()
    }

    /// Get the cache file path.
    pub fn cache_file(&self) -> &std::path::Path {
        &self.cache_file
    }

    /// Loads the cache from a JSON file in the given directory, creating a new cache if the file does not exist.
    pub fn load<P>(cache_dir: P) -> Result<Self, crate::Error>
    where
        P: AsRef<std::path::Path>,
    {
        let cache_file = cache_dir.as_ref().join(CACHE_FILE_NAME);
        if !cache_file.is_file() {
            Ok(Self::create_at(cache_file))
        } else {
            Ok(serde_json::from_str(&std::fs::read_to_string(cache_file)?)?)
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

    /// Inserts a new value into the cache, replacing any existing value with the same source digest.
    pub fn insert(&mut self, artefact: SourceArtefact) {
        self.map.insert(__digest__(artefact.source()), artefact);
    }

    /// Check whether the cache contains the given source.
    pub fn contains(&self, source: &Source) -> bool {
        self.map.contains_key(&__digest__(source))
    }

    pub fn into_maybe_cached(&self, source: Source) -> MaybeCached<Source> {
        if self.contains(&source) {
            CachedState::cached(source)
        } else {
            CachedState::not_cached(source)
        }
    }

    /// Retrieves a cached value for the given source, if it exists.
    pub fn get<'cache, 'src>(&'cache self, source: &Source) -> Option<&SourceArtefact> {
        self.map.get(&__digest__(source))
    }

    /// Removes a cached value for the given source, returning it if it existed.
    pub fn remove(&mut self, source: &Source) -> Option<SourceArtefact> {
        self.map.remove(&__digest__(source))
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
    type Item = (String, SourceArtefact);
    type IntoIter = std::collections::btree_map::IntoIter<String, SourceArtefact>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a> IntoIterator for &'a Cache {
    type Item = (&'a String, &'a SourceArtefact);
    type IntoIter = std::collections::btree_map::Iter<'a, String, SourceArtefact>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

#[cfg(test)]
mod test_cache {
    use directories::ProjectDirs;

    use super::*;

    #[test]
    fn get_digest() {
        let source = crate::source! {
            "src",
            tar = "https://example.com/foo.tar.gz"
        }
        .unwrap();
        let json = serde_json::to_string(&source).unwrap();
        let digest = __digest__(&source);
        println!("Source: {json}");
        println!("Digest: {digest}");
    }

    #[test]
    fn get_cache_dir() {
        let project_dirs = ProjectDirs::from("", "", "cargo-fetch-source").unwrap();
        let cache_dir = project_dirs.cache_dir();
        println!("Cache directory: {}", cache_dir.display());
    }

    #[test]
    fn build_cache_from_json() {
        let json = r#"
        {
            "d0f421a2f76d0a84d8f16f96f94d903a270c3b9b716384d6307f0a5046c6ff1a": {
                "path": "/path/to/source",
                "source": {
                    "git": "git@github.com:foo/bar.git",
                    "rev": "abcd1234"
                }
            }
        }"#;
        let cache: Cache = serde_json::from_str(json).unwrap();
        println!("Cache: {cache:#?}");
        assert_eq!(cache.map.len(), 1);
        // Test iteration
        for (k, v) in cache {
            assert_eq!(k, __digest__(v.source()));
        }
    }
}
