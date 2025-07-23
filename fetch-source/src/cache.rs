// A BTree maintains key order
use std::collections::BTreeMap;

use crate::Source;

const CACHE_FILE_NAME: &str = "fetch-source-cache.json";

/// Represents a cached source artefact, which includes the path to the artefact and the
/// source it was fetched from.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct CacheValue {
    pub path: std::path::PathBuf,
    pub source: crate::Source,
}

/// The cache is a collection of cached artefacts, indexed by their source's digest.
#[derive(Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Cache {
    #[serde(flatten)]
    map: BTreeMap<String, CacheValue>,
    #[serde(skip)]
    cache_file: std::path::PathBuf,
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
    pub fn insert(&mut self, artefact: crate::SourceArtefact) {
        let (artefact, source) = artefact.into_parts();
        let path = artefact.into_path();
        let digest = source.digest();
        let value = CacheValue { path, source };
        self.map.insert(digest, value);
    }

    /// Check whether the cache contains the given source.
    pub fn contains(&self, source: &Source) -> bool {
        self.map.contains_key(&source.digest())
    }

    /// Retrieves a cached value for the given source, if it exists.
    pub fn get(&self, source: &Source) -> Option<&CacheValue> {
        self.map.get(&source.digest())
    }

    /// Removes a cached value for the given source, returning it if it existed.
    pub fn remove(&mut self, source: &Source) -> Option<CacheValue> {
        self.map.remove(&source.digest())
    }

    /// Returns an iterator over the cached values.
    pub fn values(&self) -> impl Iterator<Item = &CacheValue> {
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
    type Item = (String, CacheValue);
    type IntoIter = std::collections::btree_map::IntoIter<String, CacheValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a> IntoIterator for &'a Cache {
    type Item = (&'a String, &'a CacheValue);
    type IntoIter = std::collections::btree_map::Iter<'a, String, CacheValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

#[cfg(test)]
mod test_cache {
    use directories::ProjectDirs;

    use super::*;

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
            assert_eq!(k, v.source.digest());
        }
    }
}
