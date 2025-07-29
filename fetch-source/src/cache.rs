// A BTree maintains key order
use std::collections::BTreeMap;

use crate::{NamedSourceArtefact, Source, SourceArtefact};

const CACHE_FILE_NAME: &str = "fetch-source-cache.json";

// The digest associated with a named source
pub struct NamedDigest {
    pub name: String,
    pub digest: String,
}

// pub type CachedSources<'a, K> = BTreeMap<K, MaybeCachedSource<'a>>;

/// Represents whether a given Source is cached or not.
// pub enum MaybeCachedSource<'cache> {
//     /// The source was fetched and stored in the cache.
//     Cached(Source, &'cache SourceArtefact),
//     /// The source was not found in the cache, and this is where it should be fetched to.
//     NotCached(Source, std::path::PathBuf),
// }

// impl<'a> MaybeCachedSource<'a> {
//     pub fn take_source(self) -> Source {
//         match self {
//             MaybeCachedSource::Cached(s, _) => s,
//             MaybeCachedSource::NotCached(s, _) => s,
//         }
//     }
// }

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

    /// Get the cache file path.
    pub fn cache_file(&self) -> &std::path::Path {
        &self.cache_file
    }

    /// Get the directory of the cache file
    pub fn cache_dir(&self) -> &std::path::Path {
        self.cache_file.parent().unwrap()
    }

    /// Get the path to a cached artefact
    pub fn artefact_path(&self, source: &Source) -> std::path::PathBuf {
        self.cache_dir().join(Self::digest(source))
    }

    /// Get the digest of a source
    fn digest(source: &Source) -> String {
        __digest__(source)
    }

    /// Fetch and insert missing sources. Fetched sources are consumed and become cached artefacts.
    /// Return keys to the cached artefacts. Sources which couldn't be fetched are returned
    /// via errors.
    pub fn fetch_missing<S, F>(
        &mut self,
        sources: S,
        fetch: F,
    ) -> (Vec<NamedDigest>, Vec<crate::FetchError>)
    where
        S: Iterator<Item = (String, Source)>,
        F: FnOnce(Vec<(String, Source, std::path::PathBuf)>) -> Vec<crate::FetchResult>,
    {
        let (cached, missing) = sources.fold(
            (Vec::new(), Vec::new()),
            |(mut cached, mut missing), (name, source)| {
                let digest = Self::digest(&source);
                if self.map.contains_key(&digest) {
                    cached.push(NamedDigest { name, digest })
                } else {
                    let path = self.artefact_path(&source);
                    missing.push((name, source, path))
                }
                (cached, missing)
            },
        );
        fetch(missing)
            .into_iter()
            .fold((cached, Vec::new()), |(mut cached, mut errors), result| {
                match result {
                    Ok(artefact) => cached.push(self.insert(artefact)),
                    Err(error) => errors.push(error),
                }
                (cached, errors)
            })
    }

    /// Tag cached sources with a reference to their cached artefact, and uncached sources with the
    /// path where they should be fetched to.
    // pub fn into_cached_sources<'cache, S, K>(&'cache self, sources: S) -> CachedSources<'cache, K>
    // where
    //     S: IntoIterator<Item = (K, Source)>,
    //     K: Ord + Send + Sync,
    // {
    //     sources
    //         .into_iter()
    //         .map(|(key, source)| {
    //             let maybe_cached = match self.get(&source) {
    //                 Some(artefact) => MaybeCachedSource::Cached(source, artefact),
    //                 None => {
    //                     let artefact_path = self.artefact_path(&source);
    //                     MaybeCachedSource::NotCached(source, artefact_path)
    //                 }
    //             };
    //             (key, maybe_cached)
    //         })
    //         .collect()
    // }

    /// Re-check which sources are cached. Useful after a cache update when sources have been fetched
    // pub fn refresh<'cache, K>(
    //     &'cache self,
    //     cached_sources: CachedSources<'_, K>,
    // ) -> CachedSources<'cache, K>
    // where
    //     K: Ord + Send + Sync,
    // {
    //     self.into_cached_sources(
    //         cached_sources
    //             .into_iter()
    //             .map(|(k, s)| (k, s.take_source())),
    //     )
    // }

    /// Loads the cache from a JSON file in the given directory, creating a new cache if the file does not exist.
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

    /// Inserts a new value into the cache, replacing any existing value with the same source digest.
    /// Returns the key of the newly-inserted artefact
    pub fn insert(&mut self, artefact: NamedSourceArtefact) -> NamedDigest {
        let (name, artefact) = (artefact.name, artefact.artefact);
        let digest = Self::digest(artefact.source());
        self.map.insert(digest.clone(), artefact);
        NamedDigest { name, digest }
    }

    /// Check whether the cache contains the given source.
    pub fn is_cached(&self, source: &Source) -> bool {
        self.map.contains_key(&Self::digest(source))
    }

    /// Retrieves a cached value for the given source, if it exists.
    pub fn get<'cache, 'src>(&'cache self, source: &Source) -> Option<&'cache SourceArtefact>
    where
        'src: 'cache,
    {
        self.map.get(&Self::digest(source))
    }

    /// Retrieve a cached value by the source digest
    pub fn get_digest(&self, digest: &str) -> Option<&SourceArtefact> {
        self.map.get(digest)
    }

    pub fn iter_digests<'a>(
        &'a self,
        digests: &'a [NamedDigest],
    ) -> impl Iterator<Item = (&'a str, Option<&'a SourceArtefact>)> + 'a {
        digests
            .iter()
            .map(move |digest| (digest.name.as_str(), self.get_digest(&digest.digest)))
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
