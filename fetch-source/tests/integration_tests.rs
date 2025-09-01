//! Integration tests for the fetch-source library.
//!
//! These tests use real, well-known sources to ensure the library works
//! with actual remote repositories and archives.

use fetch_source::{Cache, try_parse_toml};

/// Test that we can successfully fetch the Syn crate repository from GitHub.
/// This is a stable, well-known Git repository that should remain available.
#[test]
fn test_fetch_git_repo_syn() {
    let cargo_toml = r#"
[package.metadata.fetch-source]
syn = { git = "https://github.com/dtolnay/syn.git" }
"#;

    let sources = try_parse_toml(cargo_toml).expect("Failed to parse TOML");
    assert_eq!(sources.len(), 1);

    let syn_source = sources
        .into_iter()
        .find(|(name, _)| name == "syn")
        .expect("syn source not found")
        .1;

    // Create a temporary directory for the test
    let temp_dir = std::env::temp_dir().join("fetch-source-test-git");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).expect("Failed to clean up temp dir");
    }

    // Fetch the source
    let result = syn_source.fetch(&temp_dir);
    assert!(
        result.is_ok(),
        "Failed to fetch syn git repo: {:?}",
        result.err()
    );

    let artefact = result.unwrap();

    // Verify the artefact path exists and contains expected git repository content
    let artefact_path: &std::path::Path = artefact.as_ref();
    assert!(artefact_path.exists(), "Fetched directory does not exist");
    assert!(artefact_path.is_dir(), "Fetched path is not a directory");

    // Check for typical Rust project files that should be in the syn repo
    let cargo_toml_path = artefact_path.join("Cargo.toml");
    assert!(
        cargo_toml_path.exists(),
        "Cargo.toml not found in fetched syn repo"
    );

    let src_dir = artefact_path.join("src");
    assert!(
        src_dir.exists(),
        "src directory not found in fetched syn repo"
    );
    assert!(src_dir.is_dir(), "src is not a directory");

    // Cleanup
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

/// Test that we can successfully fetch a tar archive of the Syn crate from GitHub releases.
/// This uses a specific, stable release that should remain available.
#[cfg(feature = "tar")]
#[test]
fn test_fetch_tar_archive_syn() {
    let cargo_toml = r#"
[package.metadata.fetch-source]
"syn-1.0.109" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.109.tar.gz" }
"#;

    let sources = try_parse_toml(cargo_toml).expect("Failed to parse TOML");
    assert_eq!(sources.len(), 1);

    let syn_source = sources
        .into_iter()
        .find(|(name, _)| name == "syn-1.0.109")
        .expect("syn-1.0.109 source not found")
        .1;

    // Create a temporary directory for the test
    let temp_dir = std::env::temp_dir().join("fetch-source-test-tar");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).expect("Failed to clean up temp dir");
    }

    // Fetch the source
    let result = syn_source.fetch(&temp_dir);
    assert!(
        result.is_ok(),
        "Failed to fetch syn tar archive: {:?}",
        result.err()
    );

    let artefact = result.unwrap();

    // Verify the artefact path exists and contains expected content
    let artefact_path: &std::path::Path = artefact.as_ref();
    assert!(artefact_path.exists(), "Fetched directory does not exist");
    assert!(artefact_path.is_dir(), "Fetched path is not a directory");

    // The tar archive extracts to a subdirectory named syn-1.0.109/
    let extracted_dir = artefact_path.join("syn-1.0.109");
    assert!(
        extracted_dir.exists(),
        "Extracted syn-1.0.109 directory not found"
    );
    assert!(extracted_dir.is_dir(), "syn-1.0.109 is not a directory");

    // Check for typical Rust project files in the extracted directory
    let cargo_toml_path = extracted_dir.join("Cargo.toml");
    assert!(
        cargo_toml_path.exists(),
        "Cargo.toml not found in extracted syn archive"
    );

    let src_dir = extracted_dir.join("src");
    assert!(
        src_dir.exists(),
        "src directory not found in extracted syn archive"
    );
    assert!(src_dir.is_dir(), "src is not a directory");

    // Cleanup
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

/// Test fetching a Git repository with a specific branch.
/// This tests the branch functionality using a well-known repository.
#[test]
fn test_fetch_git_repo_with_branch() {
    let cargo_toml = r#"
[package.metadata.fetch-source]
"syn-master" = { git = "https://github.com/dtolnay/syn.git", branch = "master" }
"#;

    let sources = try_parse_toml(cargo_toml).expect("Failed to parse TOML");
    assert_eq!(sources.len(), 1);

    let syn_source = sources
        .into_iter()
        .find(|(name, _)| name == "syn-master")
        .expect("syn-master source not found")
        .1;

    // Create a temporary directory for the test
    let temp_dir = std::env::temp_dir().join("fetch-source-test-git-branch");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).expect("Failed to clean up temp dir");
    }

    // Fetch the source
    let result = syn_source.fetch(&temp_dir);
    assert!(
        result.is_ok(),
        "Failed to fetch syn git repo with branch: {:?}",
        result.err()
    );

    let artefact = result.unwrap();

    // Verify basic structure
    let artefact_path: &std::path::Path = artefact.as_ref();
    assert!(artefact_path.exists(), "Fetched directory does not exist");
    assert!(artefact_path.is_dir(), "Fetched path is not a directory");

    let cargo_toml_path = artefact_path.join("Cargo.toml");
    assert!(
        cargo_toml_path.exists(),
        "Cargo.toml not found in fetched syn repo"
    );

    // Cleanup
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

/// Test multiple sources fetching in the same operation.
/// This tests that the library can handle multiple sources correctly.
#[cfg(feature = "tar")]
#[test]
fn test_fetch_multiple_sources() {
    let cargo_toml = r#"
[package.metadata.fetch-source]
"syn-git" = { git = "https://github.com/dtolnay/syn.git" }
"syn-tar" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.109.tar.gz" }
"#;

    let sources = try_parse_toml(cargo_toml).expect("Failed to parse TOML");
    assert_eq!(sources.len(), 2);

    // Create temporary directories for the test
    let temp_dir_base = std::env::temp_dir().join("fetch-source-test-multiple");
    if temp_dir_base.exists() {
        std::fs::remove_dir_all(&temp_dir_base).expect("Failed to clean up temp dir");
    }

    // Fetch all sources using the library's convenience function
    // Note: Each source will be fetched to a subdirectory based on its name
    let results = sources
        .into_iter()
        .map(|(name, source)| {
            let source_dir = temp_dir_base.join(&name);
            source.fetch(&source_dir).map(|artefact| (name, artefact))
        })
        .collect::<Vec<_>>();

    // Both should succeed
    assert_eq!(results.len(), 2);
    for result in &results {
        assert!(
            result.is_ok(),
            "One of the fetches failed: {:?}",
            result.as_ref().err()
        );
    }

    // Verify both sources were fetched
    let git_result = results
        .iter()
        .find(|r| r.as_ref().unwrap().0 == "syn-git")
        .unwrap();
    let tar_result = results
        .iter()
        .find(|r| r.as_ref().unwrap().0 == "syn-tar")
        .unwrap();

    let git_artefact = &git_result.as_ref().unwrap().1;
    let tar_artefact = &tar_result.as_ref().unwrap().1;

    // Both should exist and be directories
    let git_path: &std::path::Path = git_artefact.as_ref();
    let tar_path: &std::path::Path = tar_artefact.as_ref();
    assert!(git_path.exists());
    assert!(git_path.is_dir());
    assert!(tar_path.exists());
    assert!(tar_path.is_dir());

    // Cleanup
    if temp_dir_base.exists() {
        std::fs::remove_dir_all(&temp_dir_base).ok();
    }
}

/// Test that sources can be cached when fetched and are available in the cache afterwards.
/// This test verifies the caching functionality works correctly with real sources.
#[test]
fn test_source_caching() {
    use std::collections::HashMap;
    let cargo_toml = r#"
[package.metadata.fetch-source]
"syn-cached" = { git = "https://github.com/dtolnay/syn.git" }
"#;

    let sources = try_parse_toml(cargo_toml).expect("Failed to parse TOML");
    assert_eq!(sources.len(), 1);

    // Create a temporary directory for the cache
    let cache_dir = std::env::temp_dir().join("fetch-source-test-cache");
    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir).expect("Failed to clean up cache dir");
    }
    std::fs::create_dir_all(&cache_dir).expect("Failed to create cache dir");

    // Load the cache (should be empty initially)
    let mut cache = Cache::load_or_create(&cache_dir).expect("Failed to load cache");
    assert!(cache.items().is_empty(), "Cache should be empty initially");

    // Check that the source is not cached yet
    let syn_source = sources
        .get("syn-cached")
        .expect("syn-cached source not found");
    assert!(
        !cache.items().contains(syn_source),
        "Source should not be cached initially"
    );

    let (cached, missing): (HashMap<_, _>, HashMap<_, _>) = sources
        .into_iter()
        .partition(|(_, s)| cache.items().contains(s));

    assert_eq!(cached.len(), 0, "No sources should be cached initially");
    assert_eq!(missing.len(), 1, "One source should be missing");
    assert!(
        missing.contains_key("syn-cached"),
        "Missing source should be among missing"
    );

    let (successes, errors): (HashMap<_, _>, HashMap<_, _>) = missing
        .into_iter()
        .map(|(n, s)| {
            let dest = cache.cache_dir().append(cache.items().relative_path(&s));
            (n, s.fetch(&*dest))
        })
        .partition(|(_, result)| result.is_ok());

    // Verify the fetch succeeded
    assert_eq!(errors.len(), 0, "No fetch errors should occur");
    assert_eq!(successes.len(), 1, "One source should be cached now");
    assert!(
        successes.contains_key("syn-cached"),
        "Cached source name should match"
    );

    for (_, result) in successes {
        cache
            .items_mut()
            .insert(result.expect("Successful fetch should return an Ok(artefact)"));
    }

    // Verify the source is now cached
    assert!(
        !cache.items().is_empty(),
        "Cache should not be empty after fetching"
    );
    assert_eq!(
        cache.items().len(),
        1,
        "Cache should contain exactly one item"
    );

    // Test that we can retrieve the cached source
    let syn_source_again = try_parse_toml(cargo_toml)
        .expect("Failed to parse TOML again")
        .into_iter()
        .find(|(name, _)| name == "syn-cached")
        .expect("syn-cached source not found")
        .1;

    assert!(
        cache.items().contains(&syn_source_again),
        "Source should be cached now"
    );

    let cached_artefact = cache
        .items()
        .get(&syn_source_again)
        .expect("Should be able to get cached artefact");

    // Verify the cached artefact path exists and contains expected content
    let cached_path: &std::path::Path = cached_artefact.as_ref();
    assert!(cached_path.exists(), "Cached artefact path should exist");
    assert!(
        cached_path.is_dir(),
        "Cached artefact should be a directory"
    );

    // Check for typical Rust project files in the cached directory
    let cargo_toml_path = cached_path.join("Cargo.toml");
    assert!(
        cargo_toml_path.exists(),
        "Cargo.toml should exist in cached syn repo"
    );

    let src_dir = cached_path.join("src");
    assert!(
        src_dir.exists(),
        "src directory should exist in cached syn repo"
    );
    assert!(src_dir.is_dir(), "src should be a directory");

    // Save the cache to disk
    cache.save().expect("Failed to save cache");

    // Verify cache file exists
    assert!(
        Cache::cache_file_exists(&cache_dir),
        "Cache file should exist after saving"
    );

    // Test that we can reload the cache and the source is still there
    let reloaded_cache = Cache::load_or_create(&cache_dir).expect("Failed to reload cache");
    assert_eq!(
        reloaded_cache.items().len(),
        1,
        "Reloaded cache should contain one item"
    );
    assert!(
        reloaded_cache.items().contains(&syn_source_again),
        "Source should still be cached after reload"
    );

    // Test partitioning again - now the source should be cached
    let sources_again = try_parse_toml(cargo_toml).expect("Failed to parse TOML for second test");

    // Manually partition again for the second test
    let (cached, missing): (HashMap<_, _>, HashMap<_, _>) = sources_again
        .into_iter()
        .partition(|(_, s)| cache.items().contains(s));

    assert_eq!(cached.len(), 1, "One source should be cached after reload");
    assert_eq!(
        missing.len(),
        0,
        "No sources should be missing after reload"
    );

    // Cleanup
    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir).ok();
    }
}

mod test_load_sources_table {
    use fetch_source::{ErrorKind, load_sources};

    #[test]
    fn from_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
            [package.metadata.fetch-source]
            "syn::latest" = { git = "https://github.com/dtolnay/syn.git" }
            "syn::1.0.0" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.0.tar.gz" }
        "#,
        )
        .unwrap();

        let sources = load_sources(&temp_dir);
        assert!(sources.is_ok());
        let sources = sources.unwrap();

        assert_eq!(sources.len(), 2);
        assert!(sources.contains_key("syn::latest"));
        assert!(sources.contains_key("syn::1.0.0"));
    }

    #[test]
    fn from_empty_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sources = load_sources(&temp_dir);
        assert!(sources.is_err());
        let error = sources.unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::Io);
    }

    #[test]
    fn no_sources_table() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
            [package]
            "name" = "test"
        "#,
        )
        .unwrap();

        let sources = load_sources(&temp_dir);
        assert!(sources.is_err());

        let error = sources.unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::Parse);
    }

    #[test]
    fn bad_sources_table() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
            [package.metadata.fetch-source]
            "syn::latest" = 123
        "#,
        )
        .unwrap();

        let sources = load_sources(&temp_dir);
        assert!(sources.is_err());

        let error = sources.unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::Parse);
    }
}

mod test_fetch_all_serial {

    use fetch_source::try_parse_toml;
    use std::collections::HashMap;

    #[test]
    fn fetch_one_good() {
        let cargo_toml = r#"
[package.metadata.fetch-source]
syn = { git = "https://github.com/dtolnay/syn.git" }
        "#;
        let sources = try_parse_toml(cargo_toml).unwrap();
        let out_dir = tempfile::tempdir().unwrap();
        let mut results: HashMap<_, _> = fetch_source::fetch_all(sources, &out_dir)
            .into_iter()
            .collect();
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("syn"));
        assert!(results.get("syn").unwrap().is_ok());
        let artefact = results.remove("syn").unwrap().unwrap();
        assert_eq!(artefact.path(), out_dir.path().join("syn"));
    }

    #[test]
    fn fetch_one_bad() {
        let cargo_toml = r#"
[package.metadata.fetch-source]
syn = { git = "https://www.example.com" }
        "#;
        let sources = try_parse_toml(cargo_toml).unwrap();
        let out_dir = tempfile::tempdir().unwrap();
        let mut results: HashMap<_, _> = fetch_source::fetch_all(sources, &out_dir)
            .into_iter()
            .collect();
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("syn"));
        assert!(results.get("syn").unwrap().is_err());
        let error = results.remove("syn").unwrap().unwrap_err();
        dbg!(&error);
    }
}

#[cfg(feature = "rayon")]
mod test_fetch_all_parallel {

    use fetch_source::try_parse_toml;
    use std::collections::HashMap;

    #[test]
    fn fetch_two_good() {
        let cargo_toml = r#"
[package.metadata.fetch-source]
"syn-git" = { git = "https://github.com/dtolnay/syn.git" }
"syn-tar" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.109.tar.gz" }
        "#;
        let sources = try_parse_toml(cargo_toml).unwrap();
        let out_dir = tempfile::tempdir().unwrap();
        let mut results: HashMap<_, _> = fetch_source::fetch_all_par(sources, &out_dir)
            .into_iter()
            .collect();
        dbg!(&results);
        assert_eq!(results.len(), 2);
        assert!(results.contains_key("syn-git"));
        assert!(results.contains_key("syn-tar"));
        let syn_git = results.remove("syn-git").unwrap().unwrap();
        assert_eq!(syn_git.path(), out_dir.path().join("syn-git"));
        let syn_tar = results.remove("syn-tar").unwrap().unwrap();
        assert_eq!(syn_tar.path(), out_dir.path().join("syn-tar"));
    }
}

#[cfg(feature = "rayon")]
mod test_cache_all_par {

    use fetch_source::*;
    use toml::toml;

    #[test]
    fn cache_two_good() {
        let cargo_toml = r#"
[package.metadata.fetch-source]
"syn-git" = { git = "https://github.com/dtolnay/syn.git" }
"syn-tar" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.109.tar.gz" }
        "#;
        let sources = try_parse_toml(cargo_toml).unwrap();
        let out_dir = tempfile::tempdir().unwrap();
        let mut cache = Cache::new(out_dir).unwrap();
        let errors = cache_all_par(&mut cache, sources);
        assert_eq!(errors.len(), 0);
        assert_eq!(cache.items().len(), 2);
        let syn_git =
            Source::parse("syn-git", toml!(git = "https://github.com/dtolnay/syn.git")).unwrap();
        assert!(cache.items().contains(&syn_git));
        let syn_tar = Source::parse(
            "syn-tar",
            toml!(tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.109.tar.gz"),
        )
        .unwrap();
        assert!(cache.items().contains(&syn_tar));
    }
}
