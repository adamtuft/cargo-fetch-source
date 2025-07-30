//! Core types for intereacting with sources declared in `Cargo.toml`.

use super::error::{FetchError, FetchErrorInner};
use super::git::{Git, GitArtefact};
#[cfg(feature = "tar")]
use super::tar::{Tar, TarArtefact};

/// Errors encountered when parsing sources from `Cargo.toml`
#[derive(Debug, thiserror::Error)]
pub enum SourceParseError {
    /// An unknown source variant was encountered.
    #[error("expected a valid source type for source '{source_name}': expected one of: {known}", known = SOURCE_VARIANTS.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))]
    VariantUnknown { source_name: String },

    /// A source has multiple variants given.
    #[error("multiple source types for source '{source_name}': expected exactly one of: {known}", known = SOURCE_VARIANTS.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))]
    VariantMultiple { source_name: String },

    /// A source has a variant which depends on a disabled feature.
    #[error("source '{source_name}' has type '{variant}' but needs disabled feature '{requires}'")]
    VariantDisabled {
        source_name: String,
        variant: String,
        requires: String,
    },

    /// A toml value was expected to be a table.
    #[error("expected value '{name}' to be a toml table")]
    ValueNotTable { name: String },

    /// The `package.metadata.fetch-source` table was not found.
    #[error("required table 'package.metadata.fetch-source' not found in string")]
    SourceTableNotFound,

    /// A toml deserialisation error occurred.
    #[error(transparent)]
    TomlInvalid(#[from] toml::de::Error),

    /// A json error occurred.
    #[error(transparent)]
    JsonInvalid(#[from] serde_json::Error),
}

pub type FetchResult = Result<SourceArtefact, crate::FetchError>;
pub type NamedFetchResult = Result<(String, SourceArtefact), crate::FetchError>;

/// Represents a source that has been fetched from a remote location.
/// This is a combination of the fetched artefact and the source it was fetched from.
/// Note that the name associated with a source *must not* be stored in the cache. This avoids
/// using one name for a source but then unexpectedly returning another.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub enum SourceArtefact {
    #[cfg(feature = "tar")]
    #[serde(rename = "tar")]
    Tar {
        source: Source,
        artefact: TarArtefact,
    },
    #[serde(rename = "git")]
    Git {
        source: Source,
        artefact: GitArtefact,
    },
}

impl AsRef<std::path::Path> for SourceArtefact {
    fn as_ref(&self) -> &std::path::Path {
        match self {
            #[cfg(feature = "tar")]
            SourceArtefact::Tar { artefact, .. } => artefact.0.as_ref(),
            SourceArtefact::Git { artefact, .. } => artefact.0.as_ref(),
        }
    }
}

impl AsRef<Source> for SourceArtefact {
    fn as_ref(&self) -> &Source {
        match self {
            #[cfg(feature = "tar")]
            SourceArtefact::Tar { source, .. } => source,
            SourceArtefact::Git { source, .. } => source,
        }
    }
}

/// Deliberately private type. This is an implementation detail only.
/// Represents the output produced when a [`Source`](crate::source::Source) is fetched.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
enum Artefact {
    #[cfg(feature = "tar")]
    #[serde(rename = "tar")]
    Tar(TarArtefact),
    #[serde(rename = "git")]
    Git(GitArtefact),
}

impl Artefact {
    fn attach_source(self, source: Source) -> SourceArtefact {
        match self {
            #[cfg(feature = "tar")]
            Artefact::Tar(artefact) => SourceArtefact::Tar { source, artefact },
            Artefact::Git(artefact) => SourceArtefact::Git { source, artefact },
        }
    }
}

/// Allowed source variants.
#[derive(Debug, PartialEq, Eq, Hash)]
enum SourceVariant {
    Tar,
    Git,
}

const SOURCE_VARIANTS: &[SourceVariant] = &[SourceVariant::Tar, SourceVariant::Git];

impl std::fmt::Display for SourceVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tar => write!(f, "tar"),
            Self::Git => write!(f, "git"),
        }
    }
}

impl SourceVariant {
    fn from<S: AsRef<str>>(name: S) -> Option<Self> {
        match name.as_ref() {
            "tar" => Some(Self::Tar),
            "git" => Some(Self::Git),
            _ => None,
        }
    }

    fn is_enabled(&self) -> bool {
        match self {
            Self::Tar => cfg!(feature = "tar"),
            Self::Git => true,
        }
    }

    fn feature(&self) -> Option<&'static str> {
        match self {
            Self::Tar => Some("tar"),
            Self::Git => None,
        }
    }
}

/// Represents an entry in the `package.metadata.fetch-source` table.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Source {
    #[cfg(feature = "tar")]
    #[serde(rename = "tar")]
    Tar(Tar),
    #[serde(rename = "git")]
    Git(Git),
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "tar")]
            Source::Tar(tar) => write!(f, "tar source: {tar:?}"),
            Source::Git(git) => write!(f, "git source: {git:?}"),
        }
    }
}

impl Source {
    /// Fetch the remote source as declared in `Cargo.toml` and put the resulting [`SourceArtefact`] in `dir`.
    pub fn fetch<P: AsRef<std::path::Path>>(self, dir: P) -> FetchResult {
        let dest = dir.as_ref();
        let result = match self {
            #[cfg(feature = "tar")]
            Source::Tar(ref tar) => tar.fetch(&dest).map(Artefact::Tar),
            Source::Git(ref git) => git.fetch(&dest).map(Artefact::Git),
        };
        match result {
            Ok(artefact) => Ok(artefact.attach_source(self)),
            Err(err) => Err(FetchError { err, source: self }),
        }
    }

    /// Convert a name into a partial path. Each `::`-separated component maps onto a subdirectory.
    pub fn as_path_component<S: AsRef<str>>(name: S) -> std::path::PathBuf {
        std::path::PathBuf::from_iter(name.as_ref().split("::"))
    }

    fn enforce_one_valid_variant<S: ToString>(
        name: S,
        source: &toml::Table,
    ) -> Result<SourceVariant, SourceParseError> {
        let mut detected_variant = None;
        for key in source.keys() {
            if let Some(variant) = SourceVariant::from(key) {
                if detected_variant.is_some() {
                    return Err(SourceParseError::VariantMultiple {
                        source_name: name.to_string(),
                    });
                }
                if !variant.is_enabled() {
                    return Err(SourceParseError::VariantDisabled {
                        source_name: name.to_string(),
                        variant: variant.to_string(),
                        requires: variant.feature().unwrap_or("?").to_string(),
                    });
                }
                detected_variant = Some(variant);
            }
        }
        detected_variant.ok_or(SourceParseError::VariantUnknown {
            source_name: name.to_string(),
        })
    }

    /// Parse a TOML table into a `Source` instance. Exactly one key in the table must identify
    /// a valid, enabled source type, otherwise an error is returned.
    pub fn parse<S: ToString>(name: S, source: toml::Table) -> Result<Self, SourceParseError> {
        Self::enforce_one_valid_variant(name, &source)?;
        Ok(toml::Value::Table(source).try_into::<Self>()?)
    }
}

/// Represents the contents of the `package.metadata.fetch-source` table in a `Cargo.toml` file.
pub type SourcesTable = std::collections::HashMap<String, Source>;

/// Parse a `package.metadata.fetch-source` table into a [`SourcesTable`](crate::source::SourcesTable) map
pub fn try_parse(table: &toml::Table) -> Result<SourcesTable, SourceParseError> {
    table
        .iter()
        .map(|(k, v)| match v.as_table() {
            Some(t) => Source::parse(k, t.to_owned()).map(|s| (k.to_owned(), s)),
            None => Err(SourceParseError::ValueNotTable { name: k.to_owned() }),
        })
        .collect()
}

/// Parse the contents of a Cargo.toml file containing the `package.metadata.fetch-source` table
/// into a [`SourcesTable`](crate::source::SourcesTable) map.
pub fn try_parse_toml<S: AsRef<str>>(toml_str: S) -> Result<SourcesTable, SourceParseError> {
    let table = toml_str.as_ref().parse::<toml::Table>()?;
    let sources_table = table
        .get("package")
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get("fetch-source"))
        .and_then(|v| v.as_table())
        .ok_or(SourceParseError::SourceTableNotFound)?;
    try_parse(sources_table)
}

#[cfg(test)]
use SourceParseError::*;

#[cfg(test)]
mod test_parsing_single_source_value {
    use super::*;
    use crate::build_from_json;

    #[test]
    fn parse_good_git_source() {
        let source = build_from_json! {
            Source,
            "git": "git@github.com:foo/bar.git"
        };
        assert!(source.is_ok());
    }

    #[cfg(feature = "tar")]
    #[test]
    fn parse_good_tar_source() {
        let source = build_from_json! {
            Source,
            "tar": "https://example.com/foo.tar.gz"
        };
        assert!(source.is_ok());
    }

    #[cfg(not(feature = "tar"))]
    #[test]
    fn parse_good_tar_source_fails_when_feature_disabled() {
        let source = build_from_json! {
            Source,
            "tar": "https://example.com/foo.tar.gz"
        };
        assert!(
            matches!(source, Err(VariantDisabled { source_name: _, variant, requires })
                if variant == "tar" && requires == "tar"
            )
        );
    }

    #[test]
    fn parse_multiple_types_fails() {
        // NOTE: this test explicitly tests failure modes of Source::parse
        let source = Source::parse(
            "src",
            toml::toml! {
                tar = "https://example.com/foo.tar.gz"
                git = "git@github.com:foo/bar.git"
            },
        );
        assert!(matches!(source, Err(VariantMultiple { source_name })
            if source_name == "src"
        ));
    }

    #[test]
    fn parse_missing_type_fails() {
        // NOTE: this test explicitly tests failure modes of Source::parse
        let source = Source::parse(
            "src",
            toml::toml! {
                foo = "git@github.com:foo/bar.git"
            },
        );
        assert!(matches!(source, Err(VariantUnknown { source_name })
            if source_name == "src"
        ));
    }
}

#[cfg(test)]
mod test_parsing_sources_table_failure_modes {
    use super::*;

    #[test]
    fn parse_invalid_toml_str_fails() {
        let document = "this is not a valid toml document :( uh-oh!";
        let result = try_parse_toml(document);
        assert!(matches!(result, Err(TomlInvalid(_))));
    }

    #[test]
    fn parse_doc_missing_sources_table_fails() {
        let document = r#"
            [package]
            name = "my_fun_test_suite"

            [package.metadata.wrong-name]
            foo = { git = "git@github.com:foo/bar.git" }
            bar = { tar = "https://example.com/foo.tar.gz" }
        "#;
        assert!(matches!(try_parse_toml(document), Err(SourceTableNotFound)));
    }

    #[test]
    fn parse_doc_source_value_not_a_table_fails() {
        let document = r#"
            [package]
            name = "my_fun_test_suite"

            [package.metadata.fetch-source]
            not-a-table = "actually a string"
        "#;
        assert!(matches!(
            try_parse_toml(document),
            Err(ValueNotTable { name }) if name == "not-a-table"
        ));
    }

    #[cfg(not(feature = "tar"))]
    #[test]
    fn parse_doc_source_variant_disabled_fails() {
        let document = r#"
            [package]
            name = "my_fun_test_suite"

            [package.metadata.fetch-source]
            bar = { tar = "https://example.com/foo.tar.gz" }
        "#;
        assert!(matches!(
            try_parse_toml(document),
            Err(VariantDisabled {
                source_name,
                variant,
                requires,
            }) if source_name == "bar" && variant == "tar" && requires == "tar"
        ));
    }

    #[test]
    fn parse_doc_source_multiple_variants_fails() {
        let document = r#"
            [package]
            name = "my_fun_test_suite"

            [package.metadata.fetch-source]
            bar = { tar = "https://example.com/foo.tar.gz", git = "git@github.com:foo/bar.git" }
        "#;
        assert!(matches!(
            try_parse_toml(document),
            Err(VariantMultiple { source_name }) if source_name == "bar"
        ));
    }

    #[test]
    fn parse_doc_source_unknown_variant_fails() {
        let document = r#"
            [package]
            name = "my_fun_test_suite"

            [package.metadata.fetch-source]
            bar = { zim = "https://example.com/foo.tar.gz" }
        "#;
        assert!(matches!(
            try_parse_toml(document),
            Err(VariantUnknown { source_name }) if source_name == "bar"
        ));
    }
}
