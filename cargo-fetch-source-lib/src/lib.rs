//! Fetch external source code dependencies defined in Cargo.toml metadata.
//!
//! This crate allows you to define external sources (Git repositories, tar archives) in your
//! `Cargo.toml` file under `[package.metadata.fetch-source]` and fetch them programmatically.
//!
//! # Features
//!
//! - **Git repositories**: Clone repositories with support for branches, tags, and specific revisions
//! - **Tar archives**: Download and extract compressed tar archives (requires `tar` feature)
//! - **Cargo.toml integration**: Define sources directly in your project metadata
//!
//! # Quick Start
//!
//! Add external sources to your `Cargo.toml`:
//!
//! ```toml
//! [package.metadata.fetch-source]
//! my-repo = { git = "https://github.com/user/repo.git", branch = "main" }
//! my-data = { tar = "https://example.com/data.tar.gz" }
//! ```
//!
//! Then fetch them in your code:
//!
//! ```rust
//! use fetch_source::{Sources, Parse};
//! use std::path::PathBuf;
//!
//! let cargo_toml = std::fs::read_to_string("Cargo.toml")?;
//! let sources = Sources::try_parse_toml(cargo_toml)?;
//!
//! for (name, source) in sources {
//!     let output_dir = PathBuf::from("./external");
//!     source.fetch(&name, output_dir)?;
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Examples
//!
//! ## Basic parsing and fetching
//!
//! ```rust
//! use fetch_source::{Sources, Parse, Artefact};
//! use std::path::PathBuf;
//!
//! let toml_content = r#"
//! [package.metadata.fetch-source]
//! example-repo = { git = "https://github.com/user/repo.git" }
//! "#;
//!
//! let sources = Sources::try_parse_toml(toml_content)?;
//! let output_dir = PathBuf::from("./downloads");
//!
//! for (name, source) in sources {
//!     match source.fetch(&name, output_dir.clone())? {
//!         Artefact::Repository(path) => {
//!             println!("Cloned repository to: {}", path.display());
//!         }
//!         Artefact::Tarball { items } => {
//!             println!("Extracted {} items from archive", items.len());
//!         }
//!     }
//! }
//! # Ok::<(), fetch_source::Error>(())
//! ```
//!
//! ## Git-specific options
//!
//! ```toml
//! [package.metadata.fetch-source]
//! # Clone a specific branch
//! feature-branch = { git = "https://github.com/user/repo.git", branch = "feature" }
//!
//! # Clone a specific tag
//! stable = { git = "https://github.com/user/repo.git", tag = "v1.0.0" }
//!
//! # Clone with submodules
//! with-deps = { git = "https://github.com/user/repo.git", recursive = true }
//! ```
//!
//! # Error Handling
//!
//! All operations return `Result` types with descriptive error messages:
//!
//! ```rust
//! use fetch_source::{Sources, Parse, Error};
//!
//! match Sources::try_parse_toml("invalid toml") {
//!     Ok(sources) => { /* ... */ }
//!     Err(Error::Source(e)) => eprintln!("TOML parsing failed: {}", e),
//!     Err(e) => eprintln!("Other error: {}", e),
//! }
//! ```

mod error;
mod git;
mod process;
mod source;
#[cfg(feature = "tar")]
mod tar;

pub use error::Error;
pub use source::{Parse, Sources, Artefact};
