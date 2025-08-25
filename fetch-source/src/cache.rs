// A BTree maintains key order
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use derive_more::Deref;

use crate::{Artefact, Digest, Source};

const CACHE_FILE_NAME: &str = "fetch-source-cache.json";

/***
NOTE: For the special path newtypes below, we derive `Deref` as this models these types as "subtypes" of
`PathBuf` i.e. they should be able to do everything a `PathBuf` can do, and have additional semantics
at certain places in the code. They model paths that are special to the `Cache` so are only
constructed in specific places, and requiring them as arguments rather than any `PathBuf` indicates
where their special meaning to the cache matters.
***/

/// The root directory of a cache
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Deref)]
pub struct CacheRoot(PathBuf);

/// The path of a cached artefact relative to the cache root
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Deref)]
pub struct RelCacheDir(PathBuf);

/// The absolute path to a cached artefact
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Deref)]
pub struct CacheDir(PathBuf);

impl CacheRoot {
    /// Get the absolute path to an artefact
    pub fn append(&self, relative: RelCacheDir) -> CacheDir {
        CacheDir(self.0.join(relative.0))
    }
}

/// Records data about the cached sources and where their artefacts are within a [`Cache`](Cache).
///
/// When a [`Source`] is fetched, insert its [`Artefact`] into a cache to avoid repeatedly fetching
/// the same source definition.
///
/// When fetching a source, check the cache subdirectory to use with [`CacheItems::relative_path`].
#[derive(Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct CacheItems {
    #[serde(flatten)]
    map: BTreeMap<Digest, Artefact>,
}

impl CacheItems {
    /// Create an empty collection.
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Retrieves a cached artefact for the given source, if it exists.
    pub fn get(&self, source: &Source) -> Option<&Artefact> {
        self.map.get(&Source::digest(source))
    }

    /// Check whether the cache contains the given source.
    pub fn contains(&self, source: &Source) -> bool {
        self.map.contains_key(&Source::digest(source))
    }

    /// Cache an artefact and return the digest of the [`Source`] which created it. Replaces any
    /// previous value for this source.
    pub fn insert(&mut self, artefact: Artefact) {
        self.map.insert(Source::digest(&artefact), artefact);
    }

    /// Removes a cached value for the given source, returning it if it existed.
    pub fn remove(&mut self, source: &Source) -> Option<Artefact> {
        self.map.remove(&Source::digest(source))
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
    pub fn relative_path<S: AsRef<Source>>(&self, source: S) -> RelCacheDir {
        RelCacheDir(PathBuf::from(Source::digest(source).as_ref()))
    }
}

/// Owns [`data`](CacheItems) about cached sources and is responsible for its persistence.
#[derive(Debug)]
pub struct Cache {
    items: CacheItems,
    cache_file: PathBuf,
}

impl Cache {
    /// Normalise to the path of a cache file. The cache dir is required to be the absolute path to
    /// the cache directory. We rely on `canonicalize` to error when the path doesn't exist.
    ///
    /// Returns an IO error if the directory doesn't exist
    #[inline]
    fn normalise_cache_file<P>(cache_dir: P) -> std::io::Result<std::path::PathBuf>
    where
        P: AsRef<Path>,
    {
        Ok(cache_dir
            .as_ref()
            .to_path_buf()
            .canonicalize()?
            .join(CACHE_FILE_NAME))
    }

    /// Create a new cache at the specified file path.
    pub fn create_at(cache_file: PathBuf) -> Self {
        Self {
            items: CacheItems::new(),
            cache_file,
        }
    }

    /// Read the cache in the given directory.
    ///
    /// Error if the directory or cache file do not exist, of if a deserialisation error occurs
    /// when reading the cache file
    pub fn read<P>(cache_dir: P) -> Result<Self, crate::Error>
    where
        P: AsRef<Path>,
    {
        let cache_file = Self::normalise_cache_file(cache_dir)?;
        let contents = std::fs::read_to_string(&cache_file)?;
        let items: CacheItems = serde_json::from_str(&contents)?;
        Ok(Self { items, cache_file })
    }

    /// Create a new cache in the given directory.
    ///
    /// Error if the directory doesn't exist, or if there is already a cache file in this directory.
    pub fn new<P>(cache_dir: P) -> Result<Self, crate::Error>
    where
        P: AsRef<Path>,
    {
        let cache_file = Self::normalise_cache_file(&cache_dir)?;
        if cache_file.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Cache file already exists",
            )
            .into());
        }
        Ok(Self::create_at(cache_file))
    }

    /// Loads the cache from a JSON file in the given directory, creating a new cache if the file
    /// does not exist. Requires that `cache_dir` exists. Note that this function doesn't
    /// actually create the cache file - this happens when the cache is saved.
    ///
    /// Returns an error if `cache_dir` doesn't exist, or if a deserialisation error occurs when
    /// reading the cache file.
    pub fn load_or_create<P>(cache_dir: P) -> Result<Self, crate::Error>
    where
        P: AsRef<Path>,
    {
        let cache_file = Self::normalise_cache_file(&cache_dir)?;
        if cache_file.is_file() {
            Self::read(cache_dir)
        } else {
            Ok(Self::create_at(cache_file))
        }
    }

    /// Saves the cache in the directory where it was created.
    ///
    /// Returns an error if a serialisation or I/O error occurs.
    pub fn save(&self) -> Result<(), crate::Error> {
        let json = serde_json::to_string_pretty(&self.items)?;
        Ok(std::fs::write(&self.cache_file, json)?)
    }

    /// Get the cache file path.
    pub fn cache_file(&self) -> &Path {
        &self.cache_file
    }

    /// Get the directory of the cache file
    pub fn cache_dir(&self) -> CacheRoot {
        CacheRoot(self.cache_file.parent().unwrap().to_path_buf())
    }

    /// Calculate the absolute path where a fetched source would be stored within the cache
    pub fn cached_path(&self, source: &Source) -> CacheDir {
        self.cache_dir().append(self.items.relative_path(source))
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
    pub fn cache_file_exists<P>(cache_dir: P) -> bool
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
    use tempfile::tempdir;

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
            PathBuf::from("/foo/bar/").join(Source::digest(&source).as_ref()),
            *cache.cached_path(&source)
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
        cache.items_mut().insert(artefact_1);
        cache.items_mut().insert(artefact_2);
        assert_eq!(cache.items().len(), 1);
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
        assert!(!items.contains(&source));

        items.insert(artefact);
        assert!(items.contains(&source));
        assert_eq!(items.len(), 1);

        let retrieved = items.get(&source).unwrap();
        assert_eq!(
            <crate::Artefact as AsRef<Path>>::as_ref(retrieved),
            Path::new("/some/path")
        );
    }

    #[test]
    fn cache_read_on_existing_dir_missing_file_fails() {
        let temp_dir = tempdir().unwrap();
        let cache_file = Cache::normalise_cache_file(&temp_dir).unwrap();
        let result = Cache::read(&temp_dir);
        assert!(!cache_file.exists(), "File shouldn't exist before test");
        assert!(result.is_err(), "Read should fail when file doesn't exist");
        assert!(
            !cache_file.exists(),
            "File shouldn't be created by `read` operation"
        );
    }

    #[test]
    fn cache_load_on_existing_dir_missing_file_gives_empty_cache() {
        let temp_dir = tempdir().unwrap();
        let cache_file = Cache::normalise_cache_file(&temp_dir).unwrap();
        assert!(!cache_file.exists(), "File shouldn't exist before test");
        let result = Cache::load_or_create(&temp_dir);
        assert!(
            result.is_ok(),
            "load_or_create should succeed when file doesn't exist"
        );
        assert!(
            !cache_file.exists(),
            "File shouldn't exist after test - only created when saved"
        );
        assert!(result.unwrap().items().is_empty());
    }

    #[test]
    fn cache_load_on_missing_dir_fails() {
        let temp_dir = std::env::temp_dir().join("1729288131-doesnt-exist-6168255555");
        assert!(
            !temp_dir.exists(),
            "The temporary directory shouldn't exist before test"
        );
        let result = Cache::load_or_create(&temp_dir);
        assert!(
            !temp_dir.exists(),
            "The temporary directory shouldn't exist after test"
        );
        assert!(
            result.is_err(),
            "load_or_create should fail when directory doesn't exist"
        );
        assert_eq!(result.unwrap_err().kind(), &crate::ErrorKind::Io);
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
        cache.items_mut().insert(artefact);

        // Save
        cache.save().unwrap();

        // Load
        let loaded_cache = Cache::load_or_create(&temp_dir).unwrap();
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
