// A BTree maintains key order
use std::collections::BTreeMap;

use crate::Source;

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct CacheValue {
    pub path: std::path::PathBuf,
    pub source: crate::Source,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Cache {
    #[serde(flatten)]
    map: BTreeMap<String, CacheValue>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn load<P>(cache_dir: P) -> Result<Self, crate::Error>
    where
        P: AsRef<std::path::Path>,
    {
        Ok(serde_json::from_str(&std::fs::read_to_string(
            cache_dir.as_ref().join("fetch-source-cache.json"),
        )?)?)
    }

    pub fn save<P>(&self, cache_dir: P) -> Result<(), crate::Error>
    where
        P: AsRef<std::path::Path>,
    {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(cache_dir.as_ref().join("fetch-source-cache.json"), json)?;
        Ok(())
    }

    pub fn insert(&mut self, value: CacheValue) {
        self.map.insert(value.source.digest(), value);
    }

    pub fn contains(&self, source: &Source) -> bool {
        self.map.contains_key(&source.digest())
    }

    pub fn get(&self, source: &Source) -> Option<&CacheValue> {
        self.map.get(&source.digest())
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.map.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &CacheValue> {
        self.map.values()
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
