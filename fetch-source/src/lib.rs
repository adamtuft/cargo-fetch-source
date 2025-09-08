#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(rustdoc::redundant_explicit_links)]
//! Declare external source dependencies in `Cargo.toml` and fetch them programmatically.
//!
//! This crate allows you to define external sources (Git repositories, tar archives) in your
//! `Cargo.toml` under `[package.metadata.fetch-source]` and fetch them programmatically.
//! This crate is intended for use in build scripts where Rust bindings are generated from external
//! source(s).
//!
//! Inspired by CMake's [`FetchContent`] module.
//!
//! [`FetchContent`]: https://cmake.org/cmake/help/latest/module/FetchContent.html#fetchcontent
//!
//! # Core Features
//!
//! - Define sources directly in your project metadata.
//! - Cache fetched sources for efficient sharing between projects.
//! - Clone git repositories (possibly recursively) by branch, tag, or specific commit (requires `git`
//!   to be installed and available on `PATH`).
//!
//! # Optional Features
//!
//! - `tar`: Download and extract `.tar.gz` archives. This is an optional feature because it uses the
//!   [`reqwest`] crate which brings quite a few more dependencies.
//! - `rayon`: Fetch sources in parallel with [`rayon`].
//!
//! [`reqwest`]: https://crates.io/crates/reqwest
//! [`rayon`]: https://crates.io/crates/rayon
//!
//! # Basic Usage
//!
//! Parse external sources declared in your `Cargo.toml` like so:
//!
//! ```rust
//! // Imagine this is in your Cargo.toml:
//! let cargo_toml = r#"
//! [package.metadata.fetch-source]
//! my-repo = { git = "https://github.com/user/repo.git", recursive = true }
//! other-repo = { git = "https://github.com/user/project.git", branch = "the-feature" }
//! my-data = { tar = "https://example.com/data.tar.gz" }
//! "#;
//!
//! for (name, source) in fetch_source::try_parse_toml(cargo_toml)? {
//!     println!("{name}: {source}");
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Fetch all sources into a directory:
//!
//! ```rust
//! # use fetch_source::Error;
//! use std::path::PathBuf;
//!
//! # fn main() -> Result<(), Error> {
//! let cargo_toml = r#"
//! [package.metadata.fetch-source]
//! "syn::latest" = { git = "https://github.com/dtolnay/syn.git" }
//! "syn::1.0.0" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.0.tar.gz" }
//! "#;
//!
//! let out_dir = PathBuf::from(std::env::temp_dir());
//! for err in fetch_source::try_parse_toml(cargo_toml)?.into_iter()
//!     .map(|(_, source)| source.fetch(&out_dir))
//!     .filter_map(Result::err) {
//!     eprintln!("{err}");
//! }
//! # Ok(())
//! # }
//! ```
//!
#![cfg_attr(
    feature = "rayon",
    doc = r##"
With `rayon`, it's trivial to fetch sources in parallel:

```rust
# use fetch_source::Error;
use rayon::prelude::*;
use std::path::PathBuf;

# fn main() -> Result<(), Error> {
let cargo_toml = r#"
[package.metadata.fetch-source]
"syn::latest" = { git = "https://github.com/dtolnay/syn.git" }
"syn::1.0.0" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.0.tar.gz" }
"#;

let out_dir = PathBuf::from(std::env::temp_dir());
fetch_source::try_parse_toml(cargo_toml)?.into_par_iter()
    .map(|(_, source)| source.fetch(&out_dir))
    .filter_map(Result::err)
    .for_each(|err| eprintln!("{err}"));
# Ok(())
# }
```
"##
)]
//!
//! # Caching Sources
//!
//! Cache sources for efficient sharing across repeated builds. Refer to the same source across
//! different builds or projects by using the same source definition in `Cargo.toml`.
//!
//! ```rust
//! # use fetch_source::Cache;
//! # fn main() -> Result<(), fetch_source::Error> {
//! let cache = Cache::load_or_create(std::env::temp_dir())?;
//!
//! let project1 = r#"
//! [package.metadata.fetch-source]
//! "syn::latest" = { git = "https://github.com/dtolnay/syn.git" }
//! "#;
//!
//! let sources1 = fetch_source::try_parse_toml(project1)?;
//! // Check where this source would be cached
//! let cache_latest = cache.cached_path(&sources1.get("syn::latest").unwrap());
//!
//! // Note the re-use of 'syn::latest' with a different definition!
//! let project2 = r#"
//! [package.metadata.fetch-source]
//! "syn::greatest" = { git = "https://github.com/dtolnay/syn.git" }
//! "syn::latest" = { git = "https://github.com/dtolnay/syn.git", branch = "dev" }
//! "#;
//!
//! let sources2 = fetch_source::try_parse_toml(project2)?;
//! let cache_greatest = cache.cached_path(&sources2.get("syn::greatest").unwrap());
//! let cache_dev = cache.cached_path(&sources2.get("syn::latest").unwrap());
//!
//! // The same source by a different name from a different project is the same in the cache
//! assert_eq!(cache_latest, cache_greatest);
//!
//! // The name doesn't uniquely identify a source - only the definition of the source matters
//! assert_ne!(cache_latest, cache_dev);
//!
//! # Ok(())
//! # }
//! ```
//!
//! # Declaring sources
//!
//! The keys in the `package.metadata.fetch-source` table name a remote source. They can include
//! any path character and zero or more '`::`' separators. Each `::`-separated component of a
//! name maps to a subdirectory of the output directory.
//!
//! Each value in the `package.metadata.fetch-source` table must be a table which identifies the
//! remote source it represents:
//!
//! **Tar archives**
//! - The `tar` key gives the URL of the archive.
//!
//! **Git repos**
//! - The `git` key gives the SSH or HTTPS upstream URL.
//! - Any one of the `branch`/`tag`/`rev` keys indicates what to clone. The default is to clone the
//!   default branch.
//! - Use `recursive = true` to recursively clone submodules.
//! - All clones are shallow, i.e. with a depth of 1.
//!

mod cache;
mod error;
mod git;
mod source;
#[cfg(feature = "tar")]
mod tar;

/// The build-time git commit hash
pub static GIT_SHA: &str = env!("VERGEN_GIT_SHA");

pub use cache::{Cache, CacheDir, CacheItems, CacheRoot, RelCacheDir};
pub use error::{Error, ErrorKind, FetchError};
pub use git::{Git, GitReference};
pub use source::{
    Artefact, Digest, FetchResult, Source, SourceName, SourceParseError, SourcesTable,
    try_parse_toml,
};
#[cfg(feature = "tar")]
pub use tar::Tar;

/// Convenience function to load sources from `Cargo.toml` in the given directory
///
/// Returns an error if the manifest can't be loaded or if deserialisation fails.
pub fn load_sources<P: AsRef<std::path::Path>>(path: P) -> Result<SourcesTable, Error> {
    Ok(try_parse_toml(&std::fs::read_to_string(
        path.as_ref().to_path_buf().join("Cargo.toml"),
    )?)?)
}

/// Convenience function to fetch all sources serially
pub fn fetch_all<P: AsRef<std::path::Path>>(
    sources: SourcesTable,
    out_dir: P,
) -> Vec<(SourceName, FetchResult<Artefact>)> {
    sources
        .into_iter()
        .map(
            |(name, source)| match source.fetch(out_dir.as_ref().join(&name)) {
                Ok(artefact) => (name, Ok(artefact)),
                Err(err) => (name, Err(err)),
            },
        )
        .collect()
}

#[cfg(feature = "rayon")]
mod par {
    use super::*;
    use rayon::prelude::*;

    /// Convenience function to fetch all sources in parallel
    pub fn fetch_all_par<P: AsRef<std::path::Path> + Sync>(
        sources: SourcesTable,
        out_dir: P,
    ) -> Vec<(SourceName, FetchResult<Artefact>)> {
        sources
            .into_par_iter()
            .map(
                |(name, source)| match source.fetch(out_dir.as_ref().join(&name)) {
                    Ok(artefact) => (name, Ok(artefact)),
                    Err(err) => (name, Err(err)),
                },
            )
            .collect::<Vec<_>>()
    }

    /// Convenience function to update the given cache with all missing sources in parallel.
    /// Returns any errors that occurred when fetching the missing sources.
    pub fn cache_all_par(
        cache: &mut Cache,
        sources: SourcesTable,
    ) -> Vec<(SourceName, FetchError)> {
        let items = cache.items();
        let cache_root = cache.cache_dir();
        let results = sources
            .into_iter()
            .filter(|(_, source)| !items.contains(source))
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|(name, source)| {
                let artefact_dir = cache_root.append(items.relative_path(&source));
                (name, source.fetch(&*artefact_dir))
            })
            .collect::<Vec<_>>();
        let items = cache.items_mut();
        results.into_iter().fold(Vec::new(), {
            |mut errors, (name, result)| {
                match result {
                    Ok(artefact) => items.insert(artefact),
                    Err(err) => errors.push((name, err)),
                }
                errors
            }
        })
    }
}

#[cfg(feature = "rayon")]
pub use par::{cache_all_par, fetch_all_par};

/// Construct a serde-compatible type from a JSON table literal. Useful in testing.
#[cfg(test)]
#[macro_export]
macro_rules! build_from_json {
    ($t:ty) => {{
        serde_json::from_value::<$t>(serde_json::json! { { } }).map_err($crate::SourceParseError::from)
    }};
    ($t:ty, $($json:tt)+) => {{
        serde_json::from_value::<$t>(serde_json::json! { { $($json)+ } }).map_err($crate::SourceParseError::from)
    }};
    ($($json:tt)*) => {{
        serde_json::from_value(serde_json::json! { { $($json)* } }).map_err($crate::SourceParseError::from)
    }};
}
