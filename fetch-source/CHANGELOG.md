# Changelog

## [0.2.0](https://github.com/adamtuft/cargo-fetch-source/compare/fetch-source-v0.1.2...fetch-source-v0.2.0) (2025-09-09)


### Features

* query cache for fetched git sources ([679c70d](https://github.com/adamtuft/cargo-fetch-source/commit/679c70d2a51d0631ad0d0b1a69c1ec8faaee4982))
* query cache for tar source given via command line ([d637732](https://github.com/adamtuft/cargo-fetch-source/commit/d63773218d2996649627bf345ada8c9391974e8c))

## [0.1.2](https://github.com/adamtuft/cargo-fetch-source/compare/fetch-source-v0.1.1...fetch-source-v0.1.2) (2025-09-01)


### ⚠ BREAKING CHANGES

* make Cache::create_at private so as not to keep

### Bug Fixes

* fetch_all, fetch_all_par fns would put all sources in the same ([5c8eecd](https://github.com/adamtuft/cargo-fetch-source/commit/5c8eecd8e3d957a50a3550361acdfcead1620c2d))
* remove unconstrained generic from cache_all_par ([5c8eecd](https://github.com/adamtuft/cargo-fetch-source/commit/5c8eecd8e3d957a50a3550361acdfcead1620c2d))


### Code Refactoring

* make Cache::create_at private so as not to keep ([5c8eecd](https://github.com/adamtuft/cargo-fetch-source/commit/5c8eecd8e3d957a50a3550361acdfcead1620c2d))


### Continuous Integration

* manually bump version ([6fcff2d](https://github.com/adamtuft/cargo-fetch-source/commit/6fcff2d4edb53aa0f9751e68e73ef8056480c2a3))

## 0.1.1 (2025-08-28)


### Continuous Integration

* manually bump version ([40ee8f8](https://github.com/adamtuft/cargo-fetch-source/commit/40ee8f8baee7e9d72c80d4393344797b7dc3d6a4))
