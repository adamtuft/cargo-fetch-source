//! Integration tests for the fetch-source library.
//!
//! These tests use real, well-known sources to ensure the library works
//! with actual remote repositories and archives.

use fetch_source::try_parse_toml;

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

    let syn_source = sources.into_iter().find(|(name, _)| name == "syn").expect("syn source not found").1;
    
    // Create a temporary directory for the test
    let temp_dir = std::env::temp_dir().join("fetch-source-test-git");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).expect("Failed to clean up temp dir");
    }

    // Fetch the source
    let result = syn_source.fetch(&temp_dir);
    assert!(result.is_ok(), "Failed to fetch syn git repo: {:?}", result.err());

    let artefact = result.unwrap();
    
    // Verify the artefact path exists and contains expected git repository content
    let artefact_path: &std::path::Path = artefact.as_ref();
    assert!(artefact_path.exists(), "Fetched directory does not exist");
    assert!(artefact_path.is_dir(), "Fetched path is not a directory");
    
    // Check for typical Rust project files that should be in the syn repo
    let cargo_toml_path = artefact_path.join("Cargo.toml");
    assert!(cargo_toml_path.exists(), "Cargo.toml not found in fetched syn repo");
    
    let src_dir = artefact_path.join("src");
    assert!(src_dir.exists(), "src directory not found in fetched syn repo");
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

    let syn_source = sources.into_iter().find(|(name, _)| name == "syn-1.0.109").expect("syn-1.0.109 source not found").1;
    
    // Create a temporary directory for the test
    let temp_dir = std::env::temp_dir().join("fetch-source-test-tar");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).expect("Failed to clean up temp dir");
    }

    // Fetch the source
    let result = syn_source.fetch(&temp_dir);
    assert!(result.is_ok(), "Failed to fetch syn tar archive: {:?}", result.err());

    let artefact = result.unwrap();
    
    // Verify the artefact path exists and contains expected content
    let artefact_path: &std::path::Path = artefact.as_ref();
    assert!(artefact_path.exists(), "Fetched directory does not exist");
    assert!(artefact_path.is_dir(), "Fetched path is not a directory");
    
    // The tar archive extracts to a subdirectory named syn-1.0.109/
    let extracted_dir = artefact_path.join("syn-1.0.109");
    assert!(extracted_dir.exists(), "Extracted syn-1.0.109 directory not found");
    assert!(extracted_dir.is_dir(), "syn-1.0.109 is not a directory");
    
    // Check for typical Rust project files in the extracted directory
    let cargo_toml_path = extracted_dir.join("Cargo.toml");
    assert!(cargo_toml_path.exists(), "Cargo.toml not found in extracted syn archive");
    
    let src_dir = extracted_dir.join("src");
    assert!(src_dir.exists(), "src directory not found in extracted syn archive");
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

    let syn_source = sources.into_iter().find(|(name, _)| name == "syn-master").expect("syn-master source not found").1;
    
    // Create a temporary directory for the test
    let temp_dir = std::env::temp_dir().join("fetch-source-test-git-branch");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).expect("Failed to clean up temp dir");
    }

    // Fetch the source
    let result = syn_source.fetch(&temp_dir);
    assert!(result.is_ok(), "Failed to fetch syn git repo with branch: {:?}", result.err());

    let artefact = result.unwrap();
    
    // Verify basic structure
    let artefact_path: &std::path::Path = artefact.as_ref();
    assert!(artefact_path.exists(), "Fetched directory does not exist");
    assert!(artefact_path.is_dir(), "Fetched path is not a directory");
    
    let cargo_toml_path = artefact_path.join("Cargo.toml");
    assert!(cargo_toml_path.exists(), "Cargo.toml not found in fetched syn repo");

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
    let results = sources.into_iter().map(|(name, source)| {
        let source_dir = temp_dir_base.join(&name);
        source.fetch(&source_dir).map(|artefact| (name, artefact))
    }).collect::<Vec<_>>();
    
    // Both should succeed
    assert_eq!(results.len(), 2);
    for result in &results {
        assert!(result.is_ok(), "One of the fetches failed: {:?}", result.as_ref().err());
    }

    // Verify both sources were fetched
    let git_result = results.iter().find(|r| r.as_ref().unwrap().0 == "syn-git").unwrap();
    let tar_result = results.iter().find(|r| r.as_ref().unwrap().0 == "syn-tar").unwrap();

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