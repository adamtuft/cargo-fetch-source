use std::collections::HashMap;
use std::path::PathBuf;

use crate::git::GitSource;
#[cfg(feature = "tar")]
use crate::tar::TarSource;

#[derive(Debug)]
pub enum Artefact {
    Tarball { items: Vec<std::path::PathBuf> },
    Repository(PathBuf),
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Source {
    #[cfg(feature = "tar")]
    Tar(TarSource),
    Git(GitSource),
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

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SourceParseError {
    #[error("expected a valid source type for source '{source_name}': expected one of: {known}", known = SourceVariant::known().iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))]
    VariantUnknown { source_name: String },
    #[error("multiple source types for source '{source_name}': expected exactly one of: {known}", known = SourceVariant::known().iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))]
    VariantMultiple { source_name: String },
    #[error("source '{source_name}' has type '{variant}' but needs disabled feature '{requires}'")]
    VariantDisabled {
        source_name: String,
        variant: String,
        requires: String,
    },
    #[error("expected value '{name}' to be a toml table")]
    ValueNotTable { name: String },
    #[error("required table 'package.metadata.fetch-source' not found in string")]
    SourceTableNotFound,
    #[error(transparent)]
    TomlInvalid(#[from] toml::de::Error),
}

impl Source {
    fn fetch(&self, name: &str, dir: PathBuf) -> Result<Artefact, crate::Error> {
        match self {
            #[cfg(feature = "tar")]
            Source::Tar(tar) => tar.fetch(name, dir),
            Source::Git(git) => git.fetch(name, dir),
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

pub type Sources = HashMap<String, Source>;

pub trait Parse {
    fn try_parse(table: &toml::Table) -> Result<Self, SourceParseError>
    where
        Self: Sized;

    fn try_parse_toml<S: AsRef<str>>(toml_str: S) -> Result<Self, SourceParseError>
    where
        Self: Sized;
}

impl Parse for Sources {
    /// Parse a `package.metadata.fetch-source` table into a `Sources` map.
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
    /// into a `Sources` map.
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

pub(crate) fn fetch_source_blocking_helper_fn<'a>(
    name: &'a str,
    source: &'a Source,
    dir: PathBuf,
) -> Result<(&'a str, Artefact), crate::Error> {
    source.fetch(name, dir).map(|artefact| (name, artefact))
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
            Err(VariantMultiple { source_name: "bar".to_string() })
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
            Err(VariantUnknown { source_name: "bar".to_string() })
        );
    }
}
