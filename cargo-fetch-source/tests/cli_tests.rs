use assert_cmd::prelude::*;
use fetch_source::{Source, SourcesTable};
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_list_command_with_missing_manifest() {
    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.args(["list", "--manifest-file", "nonexistent.toml"]);
    cmd.assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("Failed to read manifest file"));
}

#[test]
fn test_help_command_succeeds() {
    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Fetch external source trees"));
}

#[test]
fn test_version_command_succeeds() {
    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("cargo-fetch-source"));
}

#[test]
fn test_fetch_command_with_missing_out_dir() {
    let temp_dir = tempdir().unwrap();

    // Create a valid manifest file
    let manifest_path = temp_dir.path().join("Cargo.toml");
    std::fs::write(&manifest_path, "[package.metadata.fetch-source]\n").unwrap();

    let non_existent_out = temp_dir.path().join("non_existent_output");

    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.args([
        "fetch",
        "--manifest-file",
        manifest_path.to_str().unwrap(),
        "--out-dir",
        non_existent_out.to_str().unwrap(),
    ]);
    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("output directory does not exist"));
}

#[test]
fn test_fetch_command_copy() {
    let temp_dir = tempdir().unwrap();
    let manifest_path = temp_dir.path().join("Cargo.toml");
    let syn_path = temp_dir.path().join("output/syn");
    let cargo_toml = r#"
[package.metadata.fetch-source]
"syn" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.109.tar.gz" }
    "#;
    std::fs::write(&manifest_path, cargo_toml).unwrap();
    std::fs::create_dir(syn_path.parent().unwrap()).unwrap();
    assert!(!syn_path.exists());

    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.args([
        "fetch",
        "--manifest-file",
        manifest_path.to_str().unwrap(),
        "--out-dir",
        syn_path.parent().unwrap().to_str().unwrap(),
    ]);
    cmd.assert().success();
    assert!(syn_path.exists());
}

#[test]
fn test_fetch_command_no_copy() {
    let temp_dir = tempdir().unwrap();
    let syn_dir = std::path::PathBuf::from("syn-1.0.109");
    let manifest_path = temp_dir.path().join("Cargo.toml");
    let cargo_toml = r#"
[package.metadata.fetch-source]
"syn" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.109.tar.gz" }
    "#;
    std::fs::write(&manifest_path, cargo_toml).unwrap();
    assert!(!syn_dir.exists());

    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.args(["fetch", "--manifest-file", manifest_path.to_str().unwrap()]);
    cmd.assert().success();
    assert!(!syn_dir.exists());
}

#[test]
fn test_list_command_with_missing_manifest_in_cwd() {
    let temp_dir = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.current_dir(temp_dir.path());
    cmd.arg("list");
    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("could not find 'Cargo.toml'"));
}

#[test]
fn test_list_command_with_valid_manifest() {
    let temp_dir = tempdir().unwrap();

    // Create a valid manifest file with some sources
    let manifest_path = temp_dir.path().join("Cargo.toml");
    std::fs::write(
        &manifest_path,
        "[package.metadata.fetch-source]\n\
         test-source = { git = \"https://github.com/example/repo.git\" }\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.args(["list", "--manifest-file", manifest_path.to_str().unwrap()]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test-source"));
}

#[test]
fn test_list_command_with_json_format() {
    let temp_dir = tempdir().unwrap();

    // Create a valid manifest file
    let manifest_path = temp_dir.path().join("Cargo.toml");
    std::fs::write(
        &manifest_path,
        "[package.metadata.fetch-source]\n\
         test-source = { git = \"https://github.com/example/repo.git\" }\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.args([
        "list",
        "--manifest-file",
        manifest_path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.assert().success();

    // Get the JSON output and parse it as SourcesTable
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let sources_table: SourcesTable = serde_json::from_str(&stdout).unwrap();

    // Verify the expected number of sources (1)
    assert_eq!(sources_table.len(), 1);

    // Verify it contains the "test-source" entry
    assert!(sources_table.contains_key("test-source"));

    // Verify the Source has the expected git definition
    if let Some(Source::Git(git)) = sources_table.get("test-source") {
        assert_eq!(git.upstream(), "https://github.com/example/repo.git");
    } else {
        panic!("Expected test-source to be a Git source");
    }
}

#[test]
fn test_list_command_with_toml_format() {
    let temp_dir = tempdir().unwrap();

    // Create a valid manifest file
    let manifest_path = temp_dir.path().join("Cargo.toml");
    std::fs::write(
        &manifest_path,
        "[package.metadata.fetch-source]\n\
         test-source = { git = \"https://github.com/example/repo.git\" }\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.args([
        "list",
        "--manifest-file",
        manifest_path.to_str().unwrap(),
        "--format",
        "toml",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test-source"));
}

#[test]
fn test_cached_command_with_missing_cache() {
    let temp_dir = tempdir().unwrap();
    let non_existent_cache = temp_dir.path().join("non_existent_cache");

    // Cache directory should not exist initially
    assert!(!non_existent_cache.exists());

    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.args(["cached", "--cache", non_existent_cache.to_str().unwrap()]);
    // Should fail when the cache directory doesn't exist
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));

    // Cache directory should not have been created
    assert!(!non_existent_cache.exists());
}

#[test]
fn test_cached_command_query_manifest_source() {
    // Test that querying a cache for a source defined in a manifest prints the path to the cached
    // source, and excludes other cached sources.

    // First, populate the cache with two sources: one to find and one that should be excluded.
    let temp_dir = tempdir().unwrap();
    let manifest_path = temp_dir.path().join("Cargo.toml");
    let cache_path = temp_dir.path().join("cache");
    let cargo_toml = r#"
[package.metadata.fetch-source]
"syn" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.109.tar.gz" }
"syn::1.0.0" = { tar = "https://github.com/dtolnay/syn/archive/refs/tags/1.0.0.tar.gz" }
    "#;
    std::fs::write(&manifest_path, cargo_toml).unwrap();
    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.args([
        "fetch",
        "--manifest-file",
        manifest_path.to_str().unwrap(),
        "--cache",
        cache_path.to_str().unwrap(),
    ]);
    cmd.assert().success();

    // The digest of the source we want to check for in the cache
    let syn_101_digest = "ffb69c95622639cb8c8f9c0ea579954656f631bad4c4a8ed78774110337d8a1d";

    // The digest of the other source which was cached, but that shouldn't appear in the output
    let syn_100_digest = "6366d155d905264e8697cbe862fe2d8519c1d958af0e4d784b79ca89a540678b";

    // Query the cache for the "syn" source
    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.args([
        "cached",
        "--cache",
        cache_path.to_str().unwrap(),
        "--manifest-file",
        manifest_path.to_str().unwrap(),
        "--source",
        "syn",
    ]);
    let result = cmd.assert().success();
    let output = String::from_utf8_lossy(&result.get_output().stdout);

    // The output should only contain the digest of the cached "syn" source.
    assert!(output.contains(syn_101_digest));
    assert!(!output.contains(syn_100_digest));
}

#[test]
fn test_invalid_subcommand() {
    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.arg("invalid-command");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_manifest_discovery_walks_up_directory_tree() {
    let temp_dir = tempdir().unwrap();

    // Create a Cargo.toml in the root
    let manifest_path = temp_dir.path().join("Cargo.toml");
    std::fs::write(
        &manifest_path,
        "[package.metadata.fetch-source]\n\
         test-source = { git = \"https://github.com/example/repo.git\" }\n",
    )
    .unwrap();

    // Create a subdirectory
    let sub_dir = temp_dir.path().join("subdir");
    std::fs::create_dir(&sub_dir).unwrap();

    // Run command from subdirectory - should find the manifest in parent
    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.current_dir(&sub_dir);
    cmd.arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test-source"));
}

#[test]
fn test_environment_variable_detection_out_dir() {
    let temp_dir = tempdir().unwrap();
    let out_dir = temp_dir.path().join("output");
    std::fs::create_dir(&out_dir).unwrap();

    // Create a valid manifest file
    let manifest_path = temp_dir.path().join("Cargo.toml");
    std::fs::write(&manifest_path, "[package.metadata.fetch-source]\n").unwrap();

    let mut cmd = Command::cargo_bin("cargo-fetch-source").unwrap();
    cmd.env("OUT_DIR", &out_dir);
    cmd.args(["fetch", "--manifest-file", manifest_path.to_str().unwrap()]);
    // Should succeed as OUT_DIR is provided via environment
    cmd.assert().success();
}
