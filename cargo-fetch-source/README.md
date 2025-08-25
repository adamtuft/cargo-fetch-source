# Readme

Fetch and cache external source trees specified in Cargo.toml

## Installation

From crates.io (ecommended)

```bash
cargo install cargo-fetch-source
```

From source

```bash
git clone https://github.com/adamtuft/cargo-fetch-source.git
cd cargo-fetch-source
cargo install --path cargo-fetch-source
```

## Basic Usage

Declare external sources in `Cargo.toml`:

```bash
[package.metadata.fetch-source]
"syn::latest" = { git = "https://github.com/dtolnay/syn.git" }
"syn::1.0.0" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.0.tar.gz" }
```

List sources specified in a manifest file:

```bash
$ cargo fetch-source list
syn::latest:
   upstream: https://github.com/dtolnay/syn.git
   recursive: false
syn::1.0.0:
   upstream: https://github.com/dtolnay/syn/archive/refs/tags/1.0.0.tar.gz
```

Fetch declared sources:

```bash
cargo fetch-source fetch
[1/2] ✅  syn::latest -> /home/me/.cache/cargo-fetch-source/e919341bd778304b42215bfd5c9015df7113cf5addb1ad8b7bcd057887e35de3
[2/2] ✅  syn::1.0.0 -> /home/me/.cache/cargo-fetch-source/6366d155d905264e8697cbe862fe2d8519c1d958af0e4d784b79ca89a540678b
```

View all available commands and options:

```bash
$ cargo fetch-source --help
Fetch external source trees specified in Cargo.toml

Usage: fetch-source <COMMAND>

Commands:
  fetch   Fetch the sources specified in the manifest
  list    List the sources specified in the manifest without fetching them
  cached  List the cached sources. Defaults to `CARGO_FETCH_SOURCE_CACHE` environment variable then `~/.cache/cargo-fetch-source`
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
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
