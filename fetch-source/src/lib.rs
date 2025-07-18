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
//! - `async`: Enable fetching `tar` sources asynchronously.
//!
//! [`reqwest`]: https://crates.io/crates/reqwest
//!
//! # Usage
//!
//! Parse external sources declared in your `Cargo.toml` like so:
//!
//! ```rust
//! use fetch_source::{Sources, Parse};
//!
//! // Imagine this is in your Cargo.toml:
//! let cargo_toml = r#"
//! [package.metadata.fetch-source]
//! my-repo = { git = "https://github.com/user/repo.git", recursive = true }
//! other-repo = { git = "https://github.com/user/project.git", branch = "the-feature" }
//! my-data = { tar = "https://example.com/data.tar.gz" }
//! "#;
//!
//! for (name, source) in Sources::try_parse_toml(cargo_toml)? {
//!     println!("{name}: {source}");
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Fetch all sources into a directory:
//!
//! ```rust
//! # use fetch_source::Error;
//! use fetch_source::{Sources, Parse};
//! use std::path::PathBuf;
//!
//! # fn main() -> Result<(), Error> {
//! let cargo_toml = r#"
//! [package.metadata.fetch-source]
//! syn = { git = "https://github.com/dtolnay/syn.git" }
//! syn-old = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.0.tar.gz" }
//! "#;
//!
//! for (name, source) in Sources::try_parse_toml(cargo_toml)? {
//!     let output_dir = PathBuf::from(std::env::temp_dir());
//!     source.fetch(&name, output_dir)?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Structure of the `package.metadata.fetch-source` table
//!
//! Each value in this table must be a table which identifies the remote source it represents:
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
