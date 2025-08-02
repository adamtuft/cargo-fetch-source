// A BTree maintains key order
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{Artefact, Source, SourceName};

const CACHE_FILE_NAME: &str = "fetch-source-cache.json";

/// The root directory of a cache
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CacheDir(PathBuf);

impl AsRef<Path> for CacheDir {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl CacheDir {
    /// Get the absolute path to an artefact
    pub fn join(&self, relative: RelativePath) -> ArtefactPath {
        ArtefactPath(self.0.join(relative.0))
    }
}

/// The relative path of an artefact in a cache
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelativePath(PathBuf);

impl AsRef<Path> for RelativePath {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

/// The absolute path to a cached artefact
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArtefactPath(PathBuf);

impl AsRef<Path> for ArtefactPath {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

/// The digest associated with the definition of a [`Source`]
#[derive(
    Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Clone,
)]
pub struct Digest(String);

impl AsRef<str> for Digest {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl std::fmt::Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Indicates whether a given [`Source`] is present in a [`Cache`] or not. The relative path
/// is either the path of the cached artefact, or, for missing sources, the path in the cache where
/// the fetched artefact should be saved
pub enum CacheStatus {
    #[allow(missing_docs)]
    Cached(RelativePath),
    #[allow(missing_docs)]
    Missing(RelativePath),
}

/// Records data about the cached sources and where their artefacts are within a [`Cache`](Cache).
///
/// # Examples
///
/// Given a [table](crate::SourcesTable) of [`Source`](crate::Source) items, check which are cached
/// and which are missing:
///
/// ```ignore
/// use fetch_source::{Cache, SourcesTable};
///
/// let (sources, cache): (Cache, SourcesTable) = /*  */;
/// let (cached, missing) = cache.items().partition_by_status(sources);
/// for (name, digest) in cached {
///     println!("source {name} is cached under digest {digest}");
/// }
/// for (name, source, cache_location) in missing {
///     let artefact_path = cache.cache_dir().join(cache_location);
///     println!("source {name} should be cached in {artefact_path} when fetched");
/// }
/// ```
#[derive(Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct CacheItems {
    #[serde(flatten)]
    map: BTreeMap<Digest, Artefact>,
}

impl CacheItems {
    /// Create a new empty cache items collection.
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Get the digest of a source
    fn digest<S: AsRef<Source>>(source: S) -> Digest {
        Digest(sha256::digest(
            serde_json::to_string(source.as_ref())
                .expect("Serialisation of Source should never fail"),
        ))
    }

    /// Cache a named source artefact and return its digest. Replaces the previous value for this
    /// source. Note that a source need not have a unique name.
    pub fn insert(&mut self, artefact: Artefact) -> Digest {
        let digest = Self::digest(&artefact);
        self.map.insert(digest.clone(), artefact);
        digest
    }

    /// Retrieves a cached value for the given source, if it exists.
    pub fn get(&self, source: &Source) -> Option<&Artefact> {
        self.map.get(&Self::digest(source))
    }

    /// Get the artefact associated with a source's digest
    pub fn get_digest(&self, digest: &Digest) -> Option<&Artefact> {
        self.map.get(digest)
    }

    /// Check whether the cache contains the given source.
    pub fn is_cached(&self, source: &Source) -> bool {
        self.map.contains_key(&Self::digest(source))
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

    /// Get the relative path for a source within a cache directory
    fn relative_path<S: AsRef<Source>>(&self, source: S) -> RelativePath {
        RelativePath(PathBuf::from(Self::digest(source.as_ref()).as_ref()))
    }

    /// Tag a [`Source`] to indicate whether it is present in the cache, along with its digest and
    /// relative path in the cache.
    pub fn status(&self, source: &Source) -> CacheStatus {
        if self.is_cached(source) {
            CacheStatus::Cached(self.relative_path(source))
        } else {
            CacheStatus::Missing(self.relative_path(source))
        }
    }

    /// Fetch and insert missing sources. Fetched sources are consumed and become cached artefacts.
    /// Return the digests of the cached source artefacts. Sources which couldn't be fetched are
    /// returned via errors.
    pub fn fetch_missing<F, S>(
        &mut self,
        sources: S,
        cache_dir: CacheDir,
        fetch: F,
    ) -> (
        Vec<(SourceName, Digest, ArtefactPath)>,
        Vec<crate::FetchError>,
    )
    where
        S: Iterator<Item = (SourceName, Source)>,
        F: FnOnce(
            Vec<(SourceName, Source, ArtefactPath)>,
        ) -> Vec<crate::FetchResult<(SourceName, Artefact, ArtefactPath)>>,
    {
        // Split the sources by their cached status. We can then only fetch missing sources and add
        // the fetched sources to those already cached
        let (mut cached, missing) = sources.fold(
            (Vec::new(), Vec::new()),
            |(mut cached, mut missing), (name, source)| {
                let artefact_path = cache_dir.join(self.relative_path(&source));
                match self.status(&source) {
                    CacheStatus::Cached(_) => {
                        let digest = Self::digest(&source);
                        cached.push((name, digest, artefact_path));
                    }
                    CacheStatus::Missing(relative_path) => {
                        let artefact_path = cache_dir.join(relative_path);
                        missing.push((name, source, artefact_path));
                    }
                }
                (cached, missing)
            },
        );

        // Fetch outstanding sources, caching artefacts and accumulating errors
        let (fetched_results, errors) = fetch(missing).into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut cached, mut errors), result| {
                match result {
                    Ok((name, artefact, artefact_path)) => {
                        cached.push((name, self.insert(artefact), artefact_path))
                    }
                    Err(error) => errors.push(error),
                }
                (cached, errors)
            },
        );

        // Combine cached and fetched results
        cached.extend(fetched_results);

        (cached, errors)
    }
}

/// Owns [`data`](CacheItems) about cached sources and is responsible for its persistence.
#[derive(Debug)]
pub struct Cache {
    items: CacheItems,
    cache_file: PathBuf,
}

impl Cache {
    /// Create a new cache at the specified file path.
    pub fn create_at(cache_file: PathBuf) -> Self {
        Self {
            items: CacheItems::new(),
            cache_file,
        }
    }

    /// Loads the cache from a JSON file in the given directory, creating a new cache if the file
    /// does not exist. Requires that `cache_dir` exists.
    ///
    /// Returns an error if `cache_dir` doesn't exist, or if a deserialisation error occurs when
    /// reading the cache file.
    pub fn load<P>(cache_dir: P) -> Result<Self, crate::Error>
    where
        P: AsRef<Path>,
    {
        // The cache dir is required to be the absolute path to the cache directory
        let cache_dir = cache_dir.as_ref().canonicalize()?;
        let cache_file = cache_dir.join(CACHE_FILE_NAME);
        if !cache_file.is_file() {
            Ok(Self::create_at(cache_file))
        } else {
            let items: CacheItems = serde_json::from_str(&std::fs::read_to_string(&cache_file)?)?;
            Ok(Self { items, cache_file })
        }
    }

    /// Saves the cache.
    pub fn save(&self) -> Result<(), crate::Error> {
        let json = serde_json::to_string_pretty(&self.items)?;
        Ok(std::fs::write(&self.cache_file, json)?)
    }

    /// Get the cache file path.
    pub fn cache_file(&self) -> &Path {
        &self.cache_file
    }

    /// Get the directory of the cache file
    pub fn cache_dir(&self) -> CacheDir {
        CacheDir(self.cache_file.parent().unwrap().to_path_buf())
    }

    /// Calculate the absolute path where a fetched source would be stored within the cache
    pub fn cached_path(&self, source: &Source) -> ArtefactPath {
        self.cache_dir().join(self.items.relative_path(source))
    }

    /// Get a reference to the cache items.
    pub fn items(&self) -> &CacheItems {
        &self.items
    }

    /// Get a mutable reference to the cache items.
    pub fn items_mut(&mut self) -> &mut CacheItems {
        &mut self.items
    }

    /// Check whether the cache file exists in the given directory.
    pub fn exists<P>(cache_dir: P) -> bool
    where
        P: AsRef<Path>,
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

impl IntoIterator for Cache {
    type Item = (Digest, Artefact);
    type IntoIter = std::collections::btree_map::IntoIter<Digest, Artefact>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a Cache {
    type Item = (&'a Digest, &'a Artefact);
    type IntoIter = std::collections::btree_map::Iter<'a, Digest, Artefact>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.items).into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper macro for creating test caches
    macro_rules! mock_cache_at {
        ($cache_file:expr) => {{ Cache::create_at(PathBuf::from($cache_file).join(CACHE_FILE_NAME)) }};
    }

    #[test]
    fn artefact_path_is_digest() {
        // The cache should determine the path to a cached artefact relative to the cache directory,
        // where the subdirectory is the digest of the source.
        let cache = mock_cache_at! {"/foo/bar"};
        let source: Source =
            crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
        assert_eq!(
            PathBuf::from("/foo/bar/").join(CacheItems::digest(&source).as_ref()),
            cache.cached_path(&source).as_ref()
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
        let digest_1 = cache.items_mut().insert(artefact_1);
        let digest_2 = cache.items_mut().insert(artefact_2);
        assert_eq!(cache.items().len(), 1);
        assert_eq!(digest_1, digest_2);
    }

    #[test]
    fn cache_items_insert_and_get() {
        let mut items = CacheItems::new();
        let artefact: crate::Artefact = crate::build_from_json! {
            "source": { "tar": "www.example.com/test.tar.gz" },
            "path": "/some/path",
        }
        .unwrap();

        let source: Source =
            crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
        assert!(!items.is_cached(&source));

        let digest = items.insert(artefact);
        assert!(items.is_cached(&source));
        assert_eq!(items.len(), 1);

        let retrieved = items.get(&source).unwrap();
        assert_eq!(
            <crate::Artefact as AsRef<Path>>::as_ref(retrieved),
            Path::new("/some/path")
        );

        let retrieved_by_digest = items.get_digest(&digest).unwrap();
        assert_eq!(retrieved, retrieved_by_digest);
    }

    #[test]
    fn cache_load_save_roundtrip() {
        let temp_dir = std::env::temp_dir().join("cache_test_migration");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create and populate cache
        let mut cache = Cache::create_at(temp_dir.join(CACHE_FILE_NAME));
        let artefact: crate::Artefact = crate::build_from_json! {
            "source": { "tar": "www.example.com/test.tar.gz" },
            "path": "/some/path",
        }
        .unwrap();
        let _original_digest = cache.items_mut().insert(artefact);

        // Save
        cache.save().unwrap();

        // Load
        let loaded_cache = Cache::load(&temp_dir).unwrap();
        assert_eq!(loaded_cache.items().len(), 1);

        let source: Source =
            crate::build_from_json! { "tar": "www.example.com/test.tar.gz" }.unwrap();
        let loaded_artefact = loaded_cache.items().get(&source).unwrap();
        assert_eq!(
            <crate::Artefact as AsRef<Path>>::as_ref(loaded_artefact),
            Path::new("/some/path")
        );

        // Clean up
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
