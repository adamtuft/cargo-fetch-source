# `fetch-source`

Declare external source dependencies in `Cargo.toml` and fetch them programatically.

This crate allows you to define external sources (Git repositories, tar archives) in your
`Cargo.toml` under `[package.metadata.fetch-source]` and fetch them programmatically.
This crate is intended for use in build scripts where Rust bindings are generated from external
source(s).

Inspired by CMake's [FetchContent](https://cmake.org/cmake/help/latest/module/FetchContent.html#fetchcontent) module.

### Core Features

- Define sources directly in your project metadata.
- Cache fetched sources for efficient sharing between projects.
- Clone git repositories (possibly recursively) by branch, tag, or specific commit (requires `git`
  to be installed and available on `PATH`).

### Optional Features

- `tar`: Download and extract `.tar.gz` archives. This is an optional feature because it uses the
  [reqwest](https://crates.io/crates/reqwest) crate which brings quite a few more dependencies.
- `rayon`: Fetch sources in parallel with [rayon](https://crates.io/crates/rayon).

## Basic Usage

Parse external sources declared in your `Cargo.toml` like so:

```rust
let cargo_toml = r#"
[package.metadata.fetch-source]
my-repo = { git = "https://github.com/user/repo.git", recursive = true }
other-repo = { git = "https://github.com/user/project.git", branch = "the-feature" }
my-data = { tar = "https://example.com/data.tar.gz" }
"#;

for (name, source) in fetch_source::try_parse_toml(cargo_toml)? {
    println!("{name}: {source}");
}
```

Fetch all sources into a directory:

```rust
use std::path::PathBuf;

let cargo_toml = r#"
[package.metadata.fetch-source]
"syn::latest" = { git = "https://github.com/dtolnay/syn.git" }
"syn::1.0.0" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.0.tar.gz" }
"#;

let out_dir = PathBuf::from(std::env::temp_dir());
for err in fetch_source::try_parse_toml(cargo_toml)?.into_iter()
    .map(|(_, source)| source.fetch(&out_dir))
    .filter_map(Result::err) {
    eprintln!("{err}");
}
```

## License

Copyright (c) 2025 Adam Tuft

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
