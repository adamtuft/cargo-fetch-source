# `fetch-source`

Declare external source dependencies in `Cargo.toml` and fetch them programmatically.

This project provides the `fetch_source` library, allowing you to declare external source
dependencies in a manifest file and manage them programmatically. This is intended for use in build
scripts where Rust bindings are generated from some external source(s).

Inspired by CMake's [FetchContent](https://cmake.org/cmake/help/latest/module/FetchContent.html#fetchcontent) module.

## Components

- [`fetch-source`](fetch-source/README.md): the core library for declaring, fetching and caching sources
- [`cargo-fetch-source`](cargo-fetch-source/README.md): the `cargo` command for fetching and managing cached sources
  
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
