// A BTree maintains key order
use std::collections::BTreeMap;

use crate::{Artefact, Source, SourceName};

const CACHE_FILE_NAME: &str = "fetch-source-cache.json";

/// An opaque wrapper around a cache directory path that can only be constructed
/// from a valid Cache instance, emphasizing that this path has special meaning.
#[derive(Debug, Clone)]
pub struct CacheDir(std::path::PathBuf);

impl CacheDir {
    /// Create a new CacheDir wrapper. This is private to ensure it can only be
    /// created by Cache methods.
    fn new(path: &std::path::Path) -> Self {
        Self(path.to_path_buf())
    }

    /// Join this cache directory with a relative path to create an absolute artefact path.
    /// This method takes CacheRelativePath by value to emphasize the relationship between
    /// cache directories and relative paths within them.
    pub fn join(&self, relative_path: CacheRelativePath) -> ArtefactPath {
        ArtefactPath::new(self.0.join(relative_path.as_ref()))
    }
}

impl AsRef<std::path::Path> for CacheDir {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

/// An opaque wrapper around a relative path within a cache that can only be constructed
/// from CacheItems, emphasizing that this represents a cache-relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheRelativePath(std::path::PathBuf);

impl CacheRelativePath {
    /// Create a new CacheRelativePath wrapper. This is private to ensure it can only be
    /// created by CacheItems methods.
    fn new(path: std::path::PathBuf) -> Self {
        Self(path)
    }
}

impl AsRef<std::path::Path> for CacheRelativePath {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

/// An opaque wrapper around an absolute path to a cached artefact that can only be constructed
/// by joining a CacheDir with a CacheRelativePath, emphasizing the relationship between these types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtefactPath(std::path::PathBuf);

impl ArtefactPath {
    /// Create a new ArtefactPath wrapper. This is private to ensure it can only be
    /// created by CacheDir methods.
    fn new(path: std::path::PathBuf) -> Self {
        Self(path)
    }
}

impl AsRef<std::path::Path> for ArtefactPath {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Into<std::path::PathBuf> for ArtefactPath {
    fn into(self) -> std::path::PathBuf {
        self.0
    }
}

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
    pub name: SourceName,
    /// The source to be fetched
    pub source: Source,
    /// The destination for the fetched artefact
    pub path: std::path::PathBuf,
}

/// The cache is a collection of cached artefacts, indexed by their source's digest.
#[derive(Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Cache {
    #[serde(flatten)]
    map: BTreeMap<Digest, Artefact>,
    #[serde(skip)]
    cache_file: std::path::PathBuf,
}

impl std::fmt::Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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
    pub fn cache_dir(&self) -> CacheDir {
        CacheDir::new(self.cache_file.parent().unwrap())
    }

    /// Calculate the path which a fetched source would have within the cache
    pub fn cached_path(&self, source: &Source) -> std::path::PathBuf {
        self.cache_dir().as_ref().join(Self::digest(source))
    }

    /// Get the digest of a source
    fn digest(source: &Source) -> Digest {
        Digest(sha256::digest(
            serde_json::to_string(source).expect("Serialisation of Source should never fail"),
        ))
    }

    /// Partition a set of sources into those which are cached (giving their named digests) and
    /// those which are missing (giving their fetch specifications)
    pub fn partition_by_status<S>(
        &self,
        sources: S,
    ) -> (Vec<(SourceName, Digest)>, Vec<NamedFetchSpec>)
    where
        S: Iterator<Item = (SourceName, Source)>,
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
    ) -> (Vec<(SourceName, Digest)>, Vec<crate::FetchError>)
    where
        F: FnOnce(Vec<NamedFetchSpec>) -> Vec<crate::FetchResult<(SourceName, Artefact)>>,
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
    pub fn insert(&mut self, artefact: Artefact) -> Digest {
        let digest = Self::digest(artefact.as_ref());
        self.map.insert(digest.clone(), artefact);
        digest
    }

    /// Check whether the cache contains the given source.
    pub fn is_cached(&self, source: &Source) -> bool {
        self.map.contains_key(&Self::digest(source))
    }

    /// Retrieves a cached value for the given source, if it exists.
    pub fn get<'a>(&'a self, source: &Source) -> Option<&'a Artefact> {
        self.map.get(&Self::digest(source))
    }

    /// Get the artefact associated with a source's digest
    pub fn get_digest<'a>(&'a self, digest: &Digest) -> Option<&'a Artefact> {
        self.map.get(digest)
    }

    /// Removes a cached value for the given source, returning it if it existed.
    pub fn remove(&mut self, source: &Source) -> Option<Artefact> {
        self.map.remove(&Self::digest(source))
    }

    /// Returns an iterator over the cached values.
    pub fn values(&self) -> impl Iterator<Item = &Artefact> {
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
    type Item = (Digest, Artefact);
    type IntoIter = std::collections::btree_map::IntoIter<Digest, Artefact>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a> IntoIterator for &'a Cache {
    type Item = (&'a Digest, &'a Artefact);
    type IntoIter = std::collections::btree_map::Iter<'a, Digest, Artefact>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test types for the new design - these will replace the current types once refactoring is complete
    #[cfg(test)]
    mod new_design {
        use super::*;

        /// The runtime cache data structure - will be serializable and handle all runtime operations
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
        pub struct CacheItems {
            #[serde(flatten)]
            map: BTreeMap<Digest, Artefact>,
        }

        /// The persistence-aware cache - will handle loading/saving and provide access to CacheItems
        #[derive(Debug)]
        pub struct Cache {
            items: CacheItems,
            cache_file: std::path::PathBuf,
        }

        impl CacheItems {
            pub fn new() -> Self {
                Self {
                    map: BTreeMap::new(),
                }
            }

            pub fn insert(&mut self, artefact: Artefact) -> Digest {
                let digest = Self::digest(artefact.as_ref());
                self.map.insert(digest.clone(), artefact);
                digest
            }

            pub fn get(&self, source: &Source) -> Option<&Artefact> {
                self.map.get(&Self::digest(source))
            }

            pub fn get_digest(&self, digest: &Digest) -> Option<&Artefact> {
                self.map.get(digest)
            }

            pub fn is_cached(&self, source: &Source) -> bool {
                self.map.contains_key(&Self::digest(source))
            }

            pub fn remove(&mut self, source: &Source) -> Option<Artefact> {
                self.map.remove(&Self::digest(source))
            }

            pub fn values(&self) -> impl Iterator<Item = &Artefact> {
                self.map.values()
            }

            pub fn is_empty(&self) -> bool {
                self.map.is_empty()
            }

            pub fn len(&self) -> usize {
                self.map.len()
            }

            /// Get the relative path for a source within a cache directory
            pub fn relative_path(&self, source: &Source) -> CacheRelativePath {
                CacheRelativePath::new(std::path::PathBuf::from(Self::digest(source).as_ref()))
            }

            /// Get the digest of a source - this is CacheItems' responsibility for relative path calculation
            pub fn digest(source: &Source) -> Digest {
                Digest(sha256::digest(serde_json::to_string(source).expect("Serialisation of Source should never fail")))
            }

            pub fn partition_by_status<S>(&self, sources: S) -> (Vec<(SourceName, Digest)>, Vec<(SourceName, Source, CacheRelativePath)>)
            where
                S: Iterator<Item = (SourceName, Source)>,
            {
                sources.fold(
                    (Vec::new(), Vec::new()),
                    |(mut cached, mut missing), (name, source)| {
                        let digest = Self::digest(&source);
                        if self.map.contains_key(&digest) {
                            cached.push((name, digest));
                        } else {
                            let relative_path = self.relative_path(&source);
                            missing.push((name, source, relative_path));
                        }
                        (cached, missing)
                    },
                )
            }

            pub fn fetch_missing<F>(
                &mut self,
                sources: Vec<(SourceName, Source)>,
                cache_dir: CacheDir,
                fetch: F,
            ) -> (Vec<(SourceName, Digest)>, Vec<crate::FetchError>)
            where
                F: FnOnce(Vec<(SourceName, Source, ArtefactPath)>) -> Vec<crate::FetchResult<(SourceName, Artefact)>>,
            {
                // Partition sources into cached and missing using fold, directly creating ArtefactPaths
                let (cached_results, missing_sources) = sources
                    .into_iter()
                    .fold(
                        (Vec::new(), Vec::new()),
                        |(mut cached, mut missing), (name, source)| {
                            if self.is_cached(&source) {
                                // Source is already cached - get its digest and add to cached results
                                let digest = Self::digest(&source);
                                cached.push((name, digest));
                            } else {
                                // Source is missing - create ArtefactPath directly
                                let artefact_path = cache_dir.join(self.relative_path(&source));
                                missing.push((name, source, artefact_path));
                            }
                            (cached, missing)
                        },
                    );

                // Call the fetch function with the strongly-typed paths
                let (fetched_results, errors) = fetch(missing_sources).into_iter().fold(
                    (Vec::new(), Vec::new()),
                    |(mut cached, mut errors), result| {
                        match result {
                            Ok((name, artefact)) => cached.push((name, self.insert(artefact))),
                            Err(error) => errors.push(error),
                        }
                        (cached, errors)
                    },
                );

                // Combine cached and fetched results
                let mut all_results = cached_results;
                all_results.extend(fetched_results);
                
                (all_results, errors)
            }
        }

        impl Cache {
            pub fn create_at(cache_file: std::path::PathBuf) -> Self {
                Self {
                    items: CacheItems::new(),
                    cache_file,
                }
            }

            pub fn load<P>(cache_dir: P) -> Result<Self, crate::Error>
            where
                P: AsRef<std::path::Path>,
            {
                let cache_file = cache_dir.as_ref().join(CACHE_FILE_NAME);
                if !cache_file.is_file() {
                    Ok(Self::create_at(cache_file))
                } else {
                    let items: CacheItems = serde_json::from_str(&std::fs::read_to_string(&cache_file)?)?;
                    Ok(Self {
                        items,
                        cache_file,
                    })
                }
            }

            pub fn save(&self) -> Result<(), crate::Error> {
                let json = serde_json::to_string_pretty(&self.items)?;
                Ok(std::fs::write(&self.cache_file, json)?)
            }

            pub fn cache_file(&self) -> &std::path::Path {
                &self.cache_file
            }

            pub fn cache_dir(&self) -> CacheDir {
                CacheDir::new(self.cache_file.parent().unwrap())
            }

            /// Calculate the absolute path where a fetched source would be stored within the cache
            pub fn cached_path(&self, source: &Source) -> ArtefactPath {
                self.cache_dir().join(self.items.relative_path(source))
            }

            pub fn items(&self) -> &CacheItems {
                &self.items
            }

            pub fn items_mut(&mut self) -> &mut CacheItems {
                &mut self.items
            }

            pub fn exists<P>(cache_dir: P) -> bool
            where
                P: AsRef<std::path::Path>,
            {
                cache_dir.as_ref().join(CACHE_FILE_NAME).is_file()
            }
        }

        impl IntoIterator for CacheItems {
            type Item = (Digest, Artefact);
            type IntoIter = std::collections::btree_map::IntoIter<Digest, Artefact>;

            fn into_iter(self) -> Self::IntoIter {
                self.map.into_iter()
            }
        }

        impl<'a> IntoIterator for &'a CacheItems {
            type Item = (&'a Digest, &'a Artefact);
            type IntoIter = std::collections::btree_map::Iter<'a, Digest, Artefact>;

            fn into_iter(self) -> Self::IntoIter {
                self.map.iter()
            }
        }

        impl serde::Serialize for Cache {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                self.items.serialize(serializer)
            }
        }
    }

    // Helper macro for the new design
    macro_rules! mock_new_cache_at {
        ($cache_file:expr) => {{
            new_design::Cache::create_at(std::path::PathBuf::from($cache_file).join(CACHE_FILE_NAME))
        }};
    }

    // Legacy tests using current implementation
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
        let artefact_1: crate::Artefact = crate::build_from_json! {
            "source": { "tar": "www.example.com/test.tar.gz" },
            "path": "AAAAAAAA",
        }
        .unwrap();
        let artefact_2: crate::Artefact = crate::build_from_json! {
            "source": { "tar": "www.example.com/test.tar.gz" },
            "path": "BBBBBBBB",
        }
        .unwrap();
        let digest_1 = cache.insert(artefact_1);
        let digest_2 = cache.insert(artefact_2);
        assert_eq!(cache.len(), 1);
        assert_eq!(digest_1, digest_2);
    }

    // Tests for the new design
    mod new_design_tests {
        use super::new_design::{self, CacheItems};
        use super::*;

        #[test]
        fn cache_items_insert_and_get() {
            let mut items = CacheItems::new();
            let artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "path": "/some/path",
            }
            .unwrap();
            
            let source: Source = crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
            assert!(!items.is_cached(&source));
            
            let digest = items.insert(artefact);
            assert!(items.is_cached(&source));
            assert_eq!(items.len(), 1);
            
            let retrieved = items.get(&source).unwrap();
            assert_eq!(<crate::Artefact as AsRef<std::path::Path>>::as_ref(retrieved), std::path::Path::new("/some/path"));
            
            let retrieved_by_digest = items.get_digest(&digest).unwrap();
            assert_eq!(retrieved, retrieved_by_digest);
        }

        #[test]
        fn cache_items_same_source_different_paths_exists_once() {
            let mut items = CacheItems::new();
            let artefact_1: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "path": "PATH_A",
            }
            .unwrap();
            let artefact_2: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "path": "PATH_B",
            }
            .unwrap();
            
            let digest_1 = items.insert(artefact_1);
            let digest_2 = items.insert(artefact_2);
            
            assert_eq!(digest_1, digest_2);
            assert_eq!(items.len(), 1);
            
            // Should have the second path (replacement)
            let source: Source = crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
            let retrieved = items.get(&source).unwrap();
            assert_eq!(<crate::Artefact as AsRef<std::path::Path>>::as_ref(retrieved), std::path::Path::new("PATH_B"));
        }

        #[test]
        fn cache_items_partition_by_status() {
            let mut items = CacheItems::new();
            
            // Add one source to cache
            let artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/cached.tar.gz" },
                "path": "/cached/path",
            }
            .unwrap();
            let cached_digest = items.insert(artefact);
            
            // Create sources list with one cached and one missing
            let sources = vec![
                ("cached".to_string(), crate::build_from_json! { "tar": "www.example.com/cached.tar.gz" }.unwrap()),
                ("missing".to_string(), crate::build_from_json! { "tar": "www.example.com/missing.tar.gz" }.unwrap()),
            ];
            
            let (cached, missing_with_relative_paths) = items.partition_by_status(sources.into_iter());
            
            assert_eq!(cached.len(), 1);
            assert_eq!(missing_with_relative_paths.len(), 1);
            
            assert_eq!(cached[0].0, "cached");
            assert_eq!(cached[0].1, cached_digest);
            
            assert_eq!(missing_with_relative_paths[0].0, "missing");
            // The relative path should be the digest
            let expected_relative_path = items.relative_path(&missing_with_relative_paths[0].1);
            assert_eq!(missing_with_relative_paths[0].2, expected_relative_path);
        }

        #[test]
        fn cache_items_fetch_missing() {
            let mut items = CacheItems::new();
            let cache_dir = CacheDir::new(std::path::Path::new("/cache/dir"));
            
            // Create sources - the method will filter out cached ones internally
            let sources = vec![
                ("test1".to_string(), crate::build_from_json! { "tar": "www.example.com/test1.tar.gz" }.unwrap()),
                ("test2".to_string(), crate::build_from_json! { "tar": "www.example.com/test2.tar.gz" }.unwrap()),
            ];
            
            // Mock fetch function that takes strongly-typed ArtefactPath and succeeds for test1, fails for test2
            let mock_fetch = |sources: Vec<(SourceName, Source, ArtefactPath)>| -> Vec<crate::FetchResult<(SourceName, Artefact)>> {
                sources.into_iter().map(|(name, source, artefact_path)| {
                    // Verify the path is absolute and in the cache directory
                    assert!(artefact_path.as_ref().is_absolute());
                    assert!(artefact_path.as_ref().starts_with("/cache/dir"));
                    
                    if name == "test1" {
                        let artefact: crate::Artefact = crate::build_from_json! {
                            "source": { "tar": "www.example.com/test1.tar.gz" },
                            "path": artefact_path.as_ref().to_string_lossy().to_string(),
                        }.unwrap();
                        Ok((name, artefact))
                    } else {
                        Err(crate::FetchError::new(crate::error::FetchErrorKind::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test error")), source))
                    }
                }).collect()
            };
            
            let (fetched, errors) = items.fetch_missing(sources, cache_dir, mock_fetch);
            
            assert_eq!(fetched.len(), 1);
            assert_eq!(errors.len(), 1);
            assert_eq!(fetched[0].0, "test1");
            assert_eq!(items.len(), 1);
        }

        #[test]
        fn cache_items_serialization() {
            let mut items = CacheItems::new();
            let artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "path": "/some/path",
            }
            .unwrap();
            items.insert(artefact);
            
            // Should be able to serialize and deserialize
            let json = serde_json::to_string_pretty(&items).unwrap();
            let deserialized: CacheItems = serde_json::from_str(&json).unwrap();
            
            assert_eq!(items, deserialized);
        }

        #[test]
        fn cache_load_save_roundtrip() {
            let temp_dir = std::env::temp_dir().join("cache_test");
            std::fs::create_dir_all(&temp_dir).unwrap();
            
            // Create and populate cache
            let mut cache = new_design::Cache::create_at(temp_dir.join(CACHE_FILE_NAME));
            let artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "path": "/some/path",
            }
            .unwrap();
            let _original_digest = cache.items_mut().insert(artefact);
            
            // Save
            cache.save().unwrap();
            
            // Load
            let loaded_cache = new_design::Cache::load(&temp_dir).unwrap();
            assert_eq!(loaded_cache.items().len(), 1);
            
            let source: Source = crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
            let loaded_artefact = loaded_cache.items().get(&source).unwrap();
            assert_eq!(<crate::Artefact as AsRef<std::path::Path>>::as_ref(loaded_artefact), std::path::Path::new("/some/path"));
            
            // Clean up
            std::fs::remove_dir_all(&temp_dir).ok();
        }

        #[test]
        fn cache_enforces_separation_of_concerns() {
            let mut cache = mock_new_cache_at! {"/cache/dir"};
            let artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "path": "/some/path",
            }
            .unwrap();
            
            // Test access through CacheItems getter methods - this is the proper API pattern
            let source: Source = crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
            assert!(!cache.items().is_cached(&source));
            
            let digest = cache.items_mut().insert(artefact);
            assert!(cache.items().is_cached(&source));
            assert_eq!(cache.items().len(), 1);
            
            let retrieved = cache.items().get(&source).unwrap();
            assert_eq!(<crate::Artefact as AsRef<std::path::Path>>::as_ref(retrieved), std::path::Path::new("/some/path"));
            
            let retrieved_by_digest = cache.items().get_digest(&digest).unwrap();
            assert_eq!(retrieved, retrieved_by_digest);
        }

        #[test]
        fn cache_path_operations() {
            let cache = mock_new_cache_at! {"/cache/dir"};
            let source: Source = crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
            
            assert_eq!(cache.cache_file(), std::path::Path::new("/cache/dir/fetch-source-cache.json"));
            assert_eq!(cache.cache_dir().as_ref(), std::path::Path::new("/cache/dir"));
            
            // Test the proper separation: Cache combines cache_dir with CacheItems relative path
            let relative_path = cache.items().relative_path(&source);
            let expected_path = cache.cache_dir().join(relative_path);
            assert_eq!(cache.cached_path(&source), expected_path);
        }

        #[test]
        fn cache_partition_by_status_demonstrates_proper_separation() {
            let mut cache = mock_new_cache_at! {"/cache/dir"};
            
            // Add one source to cache
            let artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/cached.tar.gz" },
                "path": "/cached/path",
            }
            .unwrap();
            let cached_digest = cache.items_mut().insert(artefact);
            
            // Create sources list with one cached and one missing
            let sources = vec![
                ("cached".to_string(), crate::build_from_json! { "tar": "www.example.com/cached.tar.gz" }.unwrap()),
                ("missing".to_string(), crate::build_from_json! { "tar": "www.example.com/missing.tar.gz" }.unwrap()),
            ];
            
            // Step 1: Get runtime cache information using CacheItems
            let (cached, missing_with_relative_paths) = cache.items().partition_by_status(sources.into_iter());
            
            assert_eq!(cached.len(), 1);
            assert_eq!(missing_with_relative_paths.len(), 1);
            
            assert_eq!(cached[0].0, "cached");
            assert_eq!(cached[0].1, cached_digest);
            
            assert_eq!(missing_with_relative_paths[0].0, "missing");
            
            // Step 2: The new fetch_missing API now takes raw sources and handles filtering internally
            let cache_dir = cache.cache_dir();
            
            // We can use the simplified API - convert the missing sources back to simple (name, source) tuples
            let missing_sources: Vec<(SourceName, Source)> = missing_with_relative_paths
                .into_iter()
                .map(|(name, source, _relative_path)| (name, source))
                .collect();
            
            // Mock fetch that demonstrates ArtefactPath usage
            let mock_fetch = |sources: Vec<(SourceName, Source, ArtefactPath)>| -> Vec<crate::FetchResult<(SourceName, Artefact)>> {
                sources.into_iter().map(|(name, source, artefact_path)| {
                    // Verify the ArtefactPath is correctly constructed
                    assert!(artefact_path.as_ref().is_absolute());
                    assert!(artefact_path.as_ref().starts_with("/cache/dir"));
                    
                    // Create a mock artefact
                    let artefact: crate::Artefact = crate::build_from_json! {
                        "source": source,
                        "path": artefact_path.as_ref().to_string_lossy().to_string(),
                    }.unwrap();
                    Ok((name, artefact))
                }).collect()
            };
            
            // Step 3: Use the new simplified API
            let (fetched, _errors) = cache.items_mut().fetch_missing(missing_sources, cache_dir, mock_fetch);
            
            assert_eq!(fetched.len(), 1);
            assert_eq!(fetched[0].0, "missing");
            
            // Verify the final result using cached_path for comparison
            let missing_source = crate::build_from_json! { "tar": "www.example.com/missing.tar.gz" }.unwrap();
            let _expected_path = cache.cached_path(&missing_source);
            
            // The artefact should now be in the cache at the expected path
            assert!(cache.items().is_cached(&missing_source));
        }

        #[test]
        fn cache_serialization_compatibility() {
            let cache = mock_new_cache_at! {"/cache/dir"};
            
            // Cache should be serializable (for the cached() function in main.rs)
            let json = serde_json::to_string_pretty(&cache).unwrap();
            
            // Should serialize the items, not the cache_file path
            assert!(json.contains("{}") || json.contains("[]")); // Empty cache
            assert!(!json.contains("cache_file"));
        }

        #[test]
        fn cache_items_direct_access() {
            let mut cache = mock_new_cache_at! {"/cache/dir"};
            let artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "path": "/some/path",
            }
            .unwrap();
            
            // Test direct access to items
            cache.items_mut().insert(artefact);
            assert_eq!(cache.items().len(), 1);
            
            let source: Source = crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
            assert!(cache.items().is_cached(&source));
        }

        #[test]
        fn cache_items_iteration() {
            let mut cache = mock_new_cache_at! {"/cache/dir"};
            let artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "path": "/some/path",
            }
            .unwrap();
            let _digest = cache.items_mut().insert(artefact);
            
            // Test that we can't iterate over Cache directly - must go through CacheItems
            // This enforces the separation of concerns
            
            // Instead, we create a separate CacheItems to test owned iteration
            let mut items = CacheItems::new();
            let artefact2: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/test2.tar.gz" },
                "path": "/some/path2",
            }
            .unwrap();
            let digest2 = items.insert(artefact2);
            
            let collected: Vec<_> = items.into_iter().collect();
            assert_eq!(collected.len(), 1);
            assert_eq!(collected[0].0, digest2);
        }

        #[test]
        fn cache_items_ref_iteration_through_getter() {
            let mut cache = mock_new_cache_at! {"/cache/dir"};
            let artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/test.tar.gz" },
                "path": "/some/path",
            }
            .unwrap();
            let digest = cache.items_mut().insert(artefact);
            
            // Test reference iterator on CacheItems through getter
            let items: Vec<_> = cache.items().into_iter().collect();
            assert_eq!(items.len(), 1);
            assert_eq!(*items[0].0, digest);
        }

        #[test]
        fn cache_dir_type_safety() {
            let cache = mock_new_cache_at! {"/cache/dir"};
            
            // CacheDir can only be obtained from a valid Cache instance
            let cache_dir = cache.cache_dir();
            
            // It implements AsRef<Path> for interoperability
            let path: &std::path::Path = cache_dir.as_ref();
            assert_eq!(path, std::path::Path::new("/cache/dir"));
            
            // It can be used anywhere a Path is expected
            let joined = cache_dir.as_ref().join("some_file");
            assert_eq!(joined, std::path::PathBuf::from("/cache/dir/some_file"));
            
            // But it emphasizes that this is not just any arbitrary path -
            // it's specifically a cache directory path from a valid Cache
        }

        #[test]
        fn cache_dir_join_ergonomics() {
            let cache = mock_new_cache_at! {"/cache/dir"};
            let items = CacheItems::new();
            let source1: Source = crate::build_from_json! { "tar": "www.example.com/test1.tar.gz" }.unwrap();
            let source2: Source = crate::build_from_json! { "tar": "www.example.com/test2.tar.gz" }.unwrap();
            
            // CacheDir::join follows std::path::Path::join pattern - takes &self, not self
            let cache_dir = cache.cache_dir();
            
            // Can call join multiple times on the same CacheDir instance
            let path1 = cache_dir.join(items.relative_path(&source1));
            let path2 = cache_dir.join(items.relative_path(&source2));
            
            // Both paths should be absolute and start with the cache directory
            assert!(path1.as_ref().is_absolute());
            assert!(path2.as_ref().is_absolute());
            assert!(path1.as_ref().starts_with("/cache/dir"));
            assert!(path2.as_ref().starts_with("/cache/dir"));
            
            // This demonstrates the ergonomic improvement: no need to clone CacheDir
            // just like you don't need to clone std::path::Path when calling join()
        }

        #[test]
        fn cache_relative_path_type_safety() {
            let items = CacheItems::new();
            let source: Source = crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
            
            // CacheRelativePath can only be obtained from CacheItems
            let relative_path = items.relative_path(&source);
            
            // It implements AsRef<Path> for interoperability
            let _path: &std::path::Path = relative_path.as_ref();
            
            // It can be used anywhere a Path is expected
            let cache_dir = std::path::Path::new("/cache");
            let absolute_path = cache_dir.join(relative_path.as_ref());
            
            // But it emphasizes that this is not just any arbitrary path -
            // it's specifically a cache-relative path from CacheItems
            assert!(absolute_path.starts_with("/cache"));
            
            // The type system prevents confusion between absolute and relative paths
            // You cannot accidentally pass a CacheRelativePath where an absolute path is expected
        }

        #[test]
        fn artefact_path_type_safety() {
            let cache = mock_new_cache_at! {"/cache/dir"};
            let items = CacheItems::new();
            let source: Source = crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
            
            // ArtefactPath can only be created by joining CacheDir with CacheRelativePath
            let cache_dir = cache.cache_dir();
            let relative_path = items.relative_path(&source);
            let artefact_path = cache_dir.join(relative_path);
            
            // It implements AsRef<Path> for interoperability
            let path: &std::path::Path = artefact_path.as_ref();
            assert!(path.starts_with("/cache/dir"));
            
            // The type system enforces the relationship between CacheDir and CacheRelativePath
            // You cannot create an ArtefactPath without both components
            
            // This method signature makes the relationship crystal clear:
            // CacheDir + CacheRelativePath = ArtefactPath
            // No raw PathBuf manipulation needed
            
            // Test that it can be used where a PathBuf is expected
            let path_buf: std::path::PathBuf = artefact_path.into();
            assert!(path_buf.starts_with("/cache/dir"));
            
            // The Into trait provides ergonomic conversion when PathBuf is needed
            // This is particularly useful for APIs that expect owned PathBuf values
        }

        #[test]
        fn calling_code_pattern_for_improved_separation() {
            // Demonstrate how calling code would use the improved separation of concerns
            let mut cache = mock_new_cache_at! {"/cache/dir"};
            
            // Add one source to cache
            let artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/cached.tar.gz" },
                "path": "/cached/path",
            }
            .unwrap();
            let _cached_digest = cache.items_mut().insert(artefact);
            
            // Simulate the sources from main.rs
            let sources = vec![
                ("cached".to_string(), crate::build_from_json! { "tar": "www.example.com/cached.tar.gz" }.unwrap()),
                ("missing1".to_string(), crate::build_from_json! { "tar": "www.example.com/missing1.tar.gz" }.unwrap()),
                ("missing2".to_string(), crate::build_from_json! { "tar": "www.example.com/missing2.tar.gz" }.unwrap()),
            ];
            
            // With the simplified fetch_missing API, we can just pass all sources directly!
            let cache_dir = cache.cache_dir(); // Get the strongly-typed cache directory
            
            // Mock fetch function that works with ArtefactPath directly
            let mock_fetch = |sources: Vec<(SourceName, Source, ArtefactPath)>| -> Vec<crate::FetchResult<(SourceName, Artefact)>> {
                sources.into_iter().map(|(name, source, artefact_path)| {
                    // The fetch function receives strongly-typed ArtefactPath
                    // It can convert to PathBuf when calling Source::fetch()
                    let path_for_fetch: std::path::PathBuf = artefact_path.into();
                    
                    // Simulate successful fetch (in real code, would call source.fetch(&path_for_fetch))
                    let artefact: crate::Artefact = crate::build_from_json! {
                        "source": source,
                        "path": path_for_fetch.to_string_lossy().to_string(),
                    }.unwrap();
                    Ok((name, artefact))
                }).collect()
            };
            
            // Call fetch_missing with the simplified API - just pass all sources!
            // It will internally filter out the cached source and only fetch missing ones
            let (all_results, _errors) = cache.items_mut().fetch_missing(sources, cache_dir, mock_fetch);
            
            // Verify results: should have results for all 3 sources (1 cached + 2 fetched)
            assert_eq!(all_results.len(), 3); // All sources represented in results
            
            // Total cache should now have 3 items (1 original + 2 fetched)
            assert_eq!(cache.items().len(), 3);
            
            // This pattern demonstrates the ultimate simplification of the API:
            // - CacheItems::fetch_missing now takes simple (name, source) tuples directly
            // - It internally filters for missing sources and calculates relative paths
            // - It converts relative paths to ArtefactPath instances via CacheDir::join()
            // - It returns results for ALL input sources: cached ones with their existing digests, fetched ones with new digests
            // - Callback receives strongly-typed ArtefactPath, converts to PathBuf for Source::fetch()
            // - No manual filtering, path construction, or NamedFetchSpec creation needed
            // - The calling code just passes all sources and gets comprehensive results!
        }

        #[test]
        fn cache_dir_join_converts_relative_to_artefact_paths() {
            let items = CacheItems::new();
            let cache_dir = CacheDir::new(std::path::Path::new("/cache/dir"));
            
            // Create some sources with relative paths
            let source1: Source = crate::build_from_json! { "tar": "www.example.com/test1.tar.gz" }.unwrap();
            let source2: Source = crate::build_from_json! { "tar": "www.example.com/test2.tar.gz" }.unwrap();
            
            let sources_with_relative_paths = vec![
                ("test1".to_string(), crate::build_from_json! { "tar": "www.example.com/test1.tar.gz" }.unwrap(), items.relative_path(&source1)),
                ("test2".to_string(), crate::build_from_json! { "tar": "www.example.com/test2.tar.gz" }.unwrap(), items.relative_path(&source2)),
            ];
            
            // Test using CacheDir::join() directly in iterator mapping
            let sources_with_artefact_paths: Vec<(SourceName, Source, ArtefactPath)> = sources_with_relative_paths
                .into_iter()
                .map(|(name, source, relative_path)| {
                    (name, source, cache_dir.join(relative_path))
                })
                .collect();
            
            assert_eq!(sources_with_artefact_paths.len(), 2);
            
            // Verify the first result
            assert_eq!(sources_with_artefact_paths[0].0, "test1");
            assert!(sources_with_artefact_paths[0].2.as_ref().is_absolute());
            assert!(sources_with_artefact_paths[0].2.as_ref().starts_with("/cache/dir"));
            
            // Verify the second result
            assert_eq!(sources_with_artefact_paths[1].0, "test2");
            assert!(sources_with_artefact_paths[1].2.as_ref().is_absolute());
            assert!(sources_with_artefact_paths[1].2.as_ref().starts_with("/cache/dir"));
            
            // The paths should be different (different digests)
            assert_ne!(sources_with_artefact_paths[0].2, sources_with_artefact_paths[1].2);
        }

        #[test]
        fn cache_items_fetch_missing_simplified_api() {
            let mut items = CacheItems::new();
            let cache_dir = CacheDir::new(std::path::Path::new("/cache/dir"));
            
            // Add one source to cache to test filtering
            let cached_artefact: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/cached.tar.gz" },
                "path": "/cached/path",
            }
            .unwrap();
            let cached_digest = items.insert(cached_artefact);
            
            // Create all sources - mix of cached and missing
            let all_sources = vec![
                ("cached".to_string(), crate::build_from_json! { "tar": "www.example.com/cached.tar.gz" }.unwrap()),
                ("missing1".to_string(), crate::build_from_json! { "tar": "www.example.com/missing1.tar.gz" }.unwrap()),
                ("missing2".to_string(), crate::build_from_json! { "tar": "www.example.com/missing2.tar.gz" }.unwrap()),
            ];
            
            // Mock fetch function
            let mock_fetch = |sources: Vec<(SourceName, Source, ArtefactPath)>| -> Vec<crate::FetchResult<(SourceName, Artefact)>> {
                // Should only receive the missing sources
                assert_eq!(sources.len(), 2);
                assert_eq!(sources[0].0, "missing1");
                assert_eq!(sources[1].0, "missing2");
                
                // All paths should be absolute and in cache directory
                for (_, _, artefact_path) in &sources {
                    assert!(artefact_path.as_ref().is_absolute());
                    assert!(artefact_path.as_ref().starts_with("/cache/dir"));
                }
                
                // Simulate successful fetch for all missing sources
                sources.into_iter().map(|(name, source, artefact_path)| {
                    let artefact: crate::Artefact = crate::build_from_json! {
                        "source": source,
                        "path": artefact_path.as_ref().to_string_lossy().to_string(),
                    }.unwrap();
                    Ok((name, artefact))
                }).collect()
            };
            
            // Use the simplified API - just pass all sources!
            let (all_results, errors) = items.fetch_missing(all_sources, cache_dir, mock_fetch);
            
            // Should have results for all 3 sources (1 cached + 2 fetched)
            assert_eq!(all_results.len(), 3);
            assert_eq!(errors.len(), 0);
            
            // Check that we got results for all sources
            let result_names: std::collections::HashSet<_> = all_results.iter().map(|(name, _)| name.as_str()).collect();
            assert!(result_names.contains("cached"));
            assert!(result_names.contains("missing1"));
            assert!(result_names.contains("missing2"));
            
            // The cached source should have its original digest
            let cached_result = all_results.iter().find(|(name, _)| name == "cached").unwrap();
            assert_eq!(cached_result.1, cached_digest);
            
            // Cache should now contain 3 total items
            assert_eq!(items.len(), 3);
            
            // All sources should now be cached
            assert!(items.is_cached(&crate::build_from_json! { "tar": "www.example.com/cached.tar.gz" }.unwrap()));
            assert!(items.is_cached(&crate::build_from_json! { "tar": "www.example.com/missing1.tar.gz" }.unwrap()));
            assert!(items.is_cached(&crate::build_from_json! { "tar": "www.example.com/missing2.tar.gz" }.unwrap()));
        }

        #[test]
        fn cache_items_fetch_missing_includes_cached_sources() {
            let mut items = CacheItems::new();
            let cache_dir = CacheDir::new(std::path::Path::new("/cache/dir"));
            
            // Add some sources to cache
            let cached_artefact1: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/cached1.tar.gz" },
                "path": "/cached/path1",
            }
            .unwrap();
            let cached_artefact2: crate::Artefact = crate::build_from_json! {
                "source": { "tar": "www.example.com/cached2.tar.gz" },
                "path": "/cached/path2",
            }
            .unwrap();
            let digest1 = items.insert(cached_artefact1);
            let digest2 = items.insert(cached_artefact2);
            
            // Create a mix of cached and missing sources
            let all_sources = vec![
                ("cached1".to_string(), crate::build_from_json! { "tar": "www.example.com/cached1.tar.gz" }.unwrap()),
                ("missing".to_string(), crate::build_from_json! { "tar": "www.example.com/missing.tar.gz" }.unwrap()),
                ("cached2".to_string(), crate::build_from_json! { "tar": "www.example.com/cached2.tar.gz" }.unwrap()),
            ];
            
            // Mock fetch function - should only receive the missing source
            let mock_fetch = |sources: Vec<(SourceName, Source, ArtefactPath)>| -> Vec<crate::FetchResult<(SourceName, Artefact)>> {
                assert_eq!(sources.len(), 1);
                assert_eq!(sources[0].0, "missing");
                
                sources.into_iter().map(|(name, source, artefact_path)| {
                    let artefact: crate::Artefact = crate::build_from_json! {
                        "source": source,
                        "path": artefact_path.as_ref().to_string_lossy().to_string(),
                    }.unwrap();
                    Ok((name, artefact))
                }).collect()
            };
            
            // Call fetch_missing
            let (all_results, errors) = items.fetch_missing(all_sources, cache_dir, mock_fetch);
            
            // Should have results for all 3 sources
            assert_eq!(all_results.len(), 3);
            assert_eq!(errors.len(), 0);
            
            // Verify cached sources returned their original digests
            let cached1_result = all_results.iter().find(|(name, _)| name == "cached1").unwrap();
            let cached2_result = all_results.iter().find(|(name, _)| name == "cached2").unwrap();
            let missing_result = all_results.iter().find(|(name, _)| name == "missing").unwrap();
            
            assert_eq!(cached1_result.1, digest1);
            assert_eq!(cached2_result.1, digest2);
            // missing_result should have a different digest (newly generated)
            assert_ne!(missing_result.1, digest1);
            assert_ne!(missing_result.1, digest2);
            
            // Cache should now contain 3 items
            assert_eq!(items.len(), 3);
        }
    }
}
