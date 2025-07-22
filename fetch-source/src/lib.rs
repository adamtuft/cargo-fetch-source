#![allow(rustdoc::redundant_explicit_links)]
//! Declare external source dependencies in `Cargo.toml` and fetch them programatically.
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
//! - Clone git repositories (possibly recursively) by branch, tag, or specific commit (requires `git`
//!   to be installed and available on `PATH`).
//!
//! # Optional Features
//!
//! - `tar`: Download and extract `.tar.gz` archives. This is an optional feature because it uses the
//!   [`reqwest`] crate which brings quite a few more dependencies.
//! - `rayon`: Fetch sources in parallel with [`rayon`].
//! - `async`: Enable fetching `tar` sources asynchronously.
//!
//! [`reqwest`]: https://crates.io/crates/reqwest
//! [`rayon`]: https://crates.io/crates/rayon
//!
//! # Usage
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
//!     .map(|(name, source)| source.fetch(&name, &out_dir))
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
    .map(|(name, source)| source.fetch(&name, &out_dir))
    .filter_map(Result::err)
    .for_each(|err| eprintln!("{err}"));
# Ok(())
# }
```
"##
)]
//!
//! # Declaring sources
//!
//! The keys in the `package.metadata.fetch-source` table name a remote source. They can include
//! any path character and zero or more `::` sub-name separators. Each `::`-separated component of a
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

mod error;
pub mod git;
pub mod source;
#[cfg(feature = "tar")]
pub mod tar;

#[doc(inline)]
pub use crate::error::Error;
#[doc(inline)]
pub use crate::source::*;

/// Convenience function to load sources from `Cargo.toml` in the current directory
pub fn load_sources() -> Result<Sources, Error> {
    Ok(try_parse_toml(&std::fs::read_to_string("Cargo.toml")?)?)
}

/// Convenience function to fetch all sources serially
pub fn fetch_all<P: AsRef<std::path::Path>>(
    sources: Sources,
    out_dir: P,
) -> Vec<Result<Artefact, crate::Error>> {
    sources
        .into_iter()
        .map(|(name, source)| source.fetch(&name, &out_dir))
        .collect()
}

#[cfg(feature = "rayon")]
use rayon::prelude::*;

#[cfg(feature = "rayon")]
/// Convenience function to fetch all sources in parallel
pub fn fetch_all_par<P: AsRef<std::path::Path> + Sync>(
    sources: Sources,
    out_dir: P,
) -> Vec<Result<Artefact, crate::Error>> {
    sources
        .into_par_iter()
        .map(|(name, source)| source.fetch(&name, out_dir.as_ref()))
        .collect::<Vec<_>>()
}
