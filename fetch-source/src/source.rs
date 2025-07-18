//! Core types for intereacting with sources declared in `Cargo.toml`.

use std::collections::HashMap;
use std::path::PathBuf;

use super::error::Error;
use super::git::Git;
#[cfg(feature = "tar")]
use super::tar::{Tar, TarItems};

/// Errors encountered when parsing sources from `Cargo.toml`
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SourceParseError {
    /// An unknown source variant was encountered.
    #[error("expected a valid source type for source '{source_name}': expected one of: {known}", known = SourceVariant::known().iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))]
    VariantUnknown { source_name: String },

    /// A source has multiple variants given.
    #[error("multiple source types for source '{source_name}': expected exactly one of: {known}", known = SourceVariant::known().iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))]
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
}

/// Represents the output produced when a [`Source`](crate::source::Source) is fetched.
#[derive(Debug)]
pub enum Artefact {
    /// The items extracted from a tar archive.
    #[cfg(feature = "tar")]
    Tarball { items: TarItems },
    /// The local clone of the repo.
    Repository(PathBuf),
}

#[doc(hidden)]
impl From<TarItems> for Artefact {
    fn from(items: TarItems) -> Self {
        Self::Tarball { items }
    }
}

#[doc(hidden)]
impl From<PathBuf> for Artefact {
    fn from(repo: PathBuf) -> Self {
        Self::Repository(repo)
    }
}

/// Allowed source variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SourceVariant {
    Tar,
    Git,
}

impl std::fmt::Display for SourceVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tar => write!(f, "tar"),
            Self::Git => write!(f, "git"),
        }
    }
}

impl SourceVariant {
    /// If 'name' is a known source variant, returns the corresponding enum variant.
    /// Otherwise, returns None.
    fn from<S: AsRef<str>>(name: S) -> Option<Self> {
        match name.as_ref() {
            "tar" => Some(Self::Tar),
            "git" => Some(Self::Git),
            _ => None,
        }
    }

    const fn known() -> &'static [SourceVariant] {
        const KNOWN: &[SourceVariant] = &[SourceVariant::Tar, SourceVariant::Git];
        KNOWN
    }

    /// True if the feature for the given source variant is enabled. Defaults to
    /// true for variants not controlled by a feature flag.
    fn is_enabled(&self) -> bool {
        match self {
            Self::Tar => cfg!(feature = "tar"),
            Self::Git => true,
        }
    }

    /// Get the feature (if any) required for the source variant.
    fn feature(&self) -> Option<&'static str> {
        match self {
            Self::Tar => Some("tar"),
            Self::Git => None,
        }
    }
}

/// Represents an entry in the `package.metadata.fetch-source` table.
#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Source {
    #[cfg(feature = "tar")]
    Tar(Tar),
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
    /// Fetch the remote source as declared in `Cargo.toml` and put the resulting [`Artefact`] in `dir`.
    pub fn fetch<P: AsRef<std::path::Path>>(&self, name: &str, dir: P) -> Result<Artefact, Error> {
        match self {
            #[cfg(feature = "tar")]
            Source::Tar(tar) => tar.fetch(name, dir),
            Source::Git(git) => git.fetch(name, dir),
        }
    }

    /// The upstream URL (i.e. git repo or archive link).
    pub fn upstream(&self) -> &str {
        match self {
            #[cfg(feature = "tar")]
            Source::Tar(tar) => tar.upstream(),
            Source::Git(git) => git.upstream(),
        }
    }

    /// Get a reference to the inner tar source, if it is one.
    #[cfg(feature = "tar")]
    pub fn as_tar(&self) -> Option<&Tar> {
        if let Source::Tar(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// Get a reference to the inner git source, if it is one.
    pub fn as_git(&self) -> Option<&Git> {
        if let Source::Git(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// Parse a TOML table into a `Source` instance. Exactly one key in the table must identify
    /// a valid, enabled source type, otherwise an error is returned.
    fn parse<S: ToString>(name: S, source: &toml::Table) -> Result<Self, SourceParseError> {
        let mut detected_variant = None;
        for key in source.keys() {
            match SourceVariant::from(key) {
                None => continue,
                Some(variant) => {
                    if detected_variant.is_some() {
                        return Err(SourceParseError::VariantMultiple {
                            source_name: name.to_string(),
                        });
                    } else if !variant.is_enabled() {
                        return Err(SourceParseError::VariantDisabled {
                            source_name: name.to_string(),
                            variant: variant.to_string(),
                            requires: variant.feature().unwrap_or("?").to_string(),
                        });
                    } else {
                        detected_variant = Some(variant);
                    }
                }
            };
        }
        if detected_variant.is_none() {
            return Err(SourceParseError::VariantUnknown {
                source_name: name.to_string(),
            });
        }
        Ok(source.to_owned().try_into()?)
    }
}

/// Represents the contents of the `package.metadata.fetch-source` table in a `Cargo.toml` file.
pub type Sources = HashMap<String, Source>;

/// Extension trait used to parse a TOML table into a [`Sources`](crate::source::Sources) map. This
/// is an extension trait because [`Sources`](crate::source::Sources) is a type alias to an external
/// type.
pub trait Parse {
    /// Try to parse a `package.metadata.fetch-source` TOML table.
    fn try_parse(table: &toml::Table) -> Result<Self, SourceParseError>
    where
        Self: Sized;

    /// Try to parse the contents of a `Cargo.toml` document which is expected to contain a
    /// `package.metadata.fetch-source` table.
    fn try_parse_toml<S: AsRef<str>>(toml_str: S) -> Result<Self, SourceParseError>
    where
        Self: Sized;
}

impl Parse for Sources {
    /// Parse a `package.metadata.fetch-source` table into a into a [`Sources`](crate::source::Sources) map
    fn try_parse(table: &toml::Table) -> Result<Self, SourceParseError> {
        table
            .iter()
            .map(|(k, v)| {
                let (n, t) = validate_table(k, v)?;
                Source::parse(&n, &t).map(|s| (n, s))
            })
            .collect()
    }

    /// Parse the contents of a Cargo.toml file containing the `package.metadata.fetch-source` table
    /// into a into a [`Sources`](crate::source::Sources) map.
    fn try_parse_toml<S: AsRef<str>>(toml_str: S) -> Result<Self, SourceParseError> {
        let table = toml_str.as_ref().parse::<toml::Table>()?;
        let sources_table = table
            .get("package")
            .and_then(|v| v.get("metadata"))
            .and_then(|v| v.get("fetch-source"))
            .and_then(|v| v.as_table())
            .ok_or(SourceParseError::SourceTableNotFound)?;
        Self::try_parse(sources_table)
    }
}

/// Validate that a TOML value is a table, returning the named table
fn validate_table<S: AsRef<str>>(
    key: S,
    value: &toml::Value,
) -> Result<(String, toml::Table), SourceParseError> {
    value
        .as_table()
        .map(|t| (key.as_ref().to_string(), t.to_owned()))
        .ok_or_else(|| SourceParseError::ValueNotTable {
            name: key.as_ref().to_string(),
        })
}

#[cfg(test)]
use SourceParseError::*;

#[cfg(test)]
mod test_parsing_single_source_value {
    use super::*;

    #[test]
    fn parse_good_git_source() {
        let table = toml::toml! {
            git = "git@github.com:foo/bar.git"
        };
        let source = Source::parse("src", &table);
        assert!(source.is_ok());
    }

    #[cfg(feature = "tar")]
    #[test]
    fn parse_good_tar_source() {
        let table = toml::toml! {
            tar = "https://example.com/foo.tar.gz"
        };
        let source = Source::parse("src", &table);
        assert!(source.is_ok());
    }

    #[cfg(not(feature = "tar"))]
    #[test]
    fn parse_good_tar_source_fails_when_feature_disabled() {
        let table = toml::toml! {
            tar = "https://example.com/foo.tar.gz"
        };
        let source = Source::parse("src", &table);
        assert!(
            matches!(source, Err(VariantDisabled { source_name, variant, requires })
                if source_name == "src" && variant == "tar" && requires == "tar"
            )
        );
    }

    #[test]
    fn parse_multiple_types_fails() {
        let table = toml::toml! {
            tar = "https://example.com/foo.tar.gz"
            git = "git@github.com:foo/bar.git"
        };
        let source = Source::parse("src", &table);
        assert!(matches!(source, Err(VariantMultiple { source_name })
            if source_name == "src"
        ));
    }

    #[test]
    fn parse_missing_type_fails() {
        let table = toml::toml! {
            foo = "git@github.com:foo/bar.git"
        };
        let source = Source::parse("src", &table);
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
        let result = Sources::try_parse_toml(document);
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
        assert_eq!(Sources::try_parse_toml(document), Err(SourceTableNotFound));
    }

    #[test]
    fn parse_doc_source_value_not_a_table_fails() {
        let document = r#"
            [package]
            name = "my_fun_test_suite"

            [package.metadata.fetch-source]
            not-a-table = "actually a string"
        "#;
        assert_eq!(
            Sources::try_parse_toml(document),
            Err(ValueNotTable {
                name: "not-a-table".to_string()
            })
        );
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
        assert_eq!(
            Sources::try_parse_toml(document),
            Err(VariantDisabled {
                source_name: "bar".to_string(),
                variant: "tar".to_string(),
                requires: "tar".to_string()
            })
        );
    }

    #[test]
    fn parse_doc_source_multiple_variants_fails() {
        let document = r#"
            [package]
            name = "my_fun_test_suite"

            [package.metadata.fetch-source]
            bar = { tar = "https://example.com/foo.tar.gz", git = "git@github.com:foo/bar.git" }
        "#;
        assert_eq!(
            Sources::try_parse_toml(document),
            Err(VariantMultiple {
                source_name: "bar".to_string()
            })
        );
    }

    #[test]
    fn parse_doc_source_unknown_variant_fails() {
        let document = r#"
            [package]
            name = "my_fun_test_suite"

            [package.metadata.fetch-source]
            bar = { zim = "https://example.com/foo.tar.gz" }
        "#;
        assert_eq!(
            Sources::try_parse_toml(document),
            Err(VariantUnknown {
                source_name: "bar".to_string()
            })
        );
    }
}
