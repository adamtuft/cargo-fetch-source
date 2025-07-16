use std::collections::HashMap;
use std::path::PathBuf;

use crate::artefact::Artefact;
use crate::git::GitSource;
#[cfg(feature = "tar")]
use crate::tar::TarSource;

#[derive(Debug, serde::Deserialize)]
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

#[derive(Debug, thiserror::Error)]
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
    #[error("source '{source_name}' is not a toml table")]
    SourceNotTable { source_name: String },
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
}

impl Source {
    fn fetch(&self, name: &str, dir: PathBuf) -> Result<Artefact, crate::Error> {
        match self {
            #[cfg(feature = "tar")]
            Source::Tar(tar) => tar.fetch(name, dir),
            Source::Git(git) => git.fetch(name, dir),
        }
    }

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

pub trait ParseTomlTable {
    fn try_parse(table: &toml::Table) -> Result<Self, SourceParseError>
    where
        Self: Sized;
}

impl ParseTomlTable for Sources {
    /// Takes as input the `package.metadata.fetch-source` table from a Cargo.toml file,
    /// and returns a `Sources` map
    fn try_parse(table: &toml::Table) -> Result<Self, SourceParseError> {
        let table_map = validate_and_convert_to_tables(table)?;
        parse_sources_from_tables(&table_map)
    }
}

impl TryFrom<String> for Sources {
    type Error = SourceParseError;

    /// Extracts a `Sources` map from a Cargo.toml document
    fn try_from(toml_str: String) -> Result<Self, Self::Error> {
        let table: toml::Table = toml_str.parse()?;
        Self::try_parse(&table)
    }
}

/// Validate that all values in the TOML table are themselves tables,
/// and convert to a Map<String, Table>
fn validate_and_convert_to_tables(
    table: &toml::Table,
) -> Result<toml::map::Map<String, toml::Table>, SourceParseError> {
    table
        .iter()
        .map(|(k, v)| {
            v.as_table()
                .map(|t| (k.clone(), t.clone()))
                .ok_or_else(|| SourceParseError::SourceNotTable {
                    source_name: k.clone(),
                })
        })
        .collect()
}

/// Parse each table entry into a Source and build the final Sources map
fn parse_sources_from_tables(
    table_map: &toml::map::Map<String, toml::Table>,
) -> Result<Sources, SourceParseError> {
    table_map
        .iter()
        .map(|(name, table)| Source::parse(name, table).map(|source| (name.clone(), source)))
        .collect()
}

pub(crate) fn get_remote_sources_from_toml_table(
    table: &toml::Table,
) -> Result<Sources, SourceParseError> {
    let sources_table = table
        .get("package")
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get("fetch-source"))
        .and_then(|v| v.as_table())
        .unwrap();

    Sources::try_parse(sources_table)
}

pub(crate) fn fetch_source_blocking<'a>(
    name: &'a str,
    source: &'a Source,
    dir: PathBuf,
) -> Result<(&'a str, Artefact), crate::Error> {
    source.fetch(name, dir).map(|artefact| (name, artefact))
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::Value;

    const GOOD_GIT_SOURCE_TABLE: &str = "{ git = 'git@github.com:foo/bar.git' }";
    const GOOD_TAR_SOURCE_TABLE: &str = "{ tar = 'https://example.com/foo.tar.gz' }";
    const BAD_MULTIPLE_TYPES: &str =
        "{ tar = 'https://example.com/foo.tar.gz', git = 'git@github.com:foo/bar.git' }";
    const BAD_MISSING_TYPE: &str = "{ foo = 'git@github.com:foo/bar.git' }";

    fn as_table(valid_toml: &str) -> Option<toml::Table> {
        valid_toml.parse::<Value>().unwrap()
            .as_table()
            .cloned()
    }

    #[test]
    fn parse_good_git_source() {
        let table = as_table(GOOD_GIT_SOURCE_TABLE).unwrap();
        let source = Source::parse("src", &table);
        assert!(source.is_ok());
    }

    #[cfg(feature = "tar")]
    #[test]
    fn parse_good_tar_source() {
        let table = as_table(GOOD_TAR_SOURCE_TABLE).unwrap();
        let source = Source::parse("src", &table);
        assert!(source.is_ok());
    }

    #[cfg(not(feature = "tar"))]
    #[test]
    fn parse_good_tar_source_fails_when_feature_disabled() {
        use SourceParseError::VariantDisabled;
        let table = as_table(GOOD_TAR_SOURCE_TABLE).unwrap();
        let source = Source::parse("src", &table);
        assert!(
            matches!(source, Err(VariantDisabled { source_name, variant, requires })
                if source_name == "src" && variant == "tar" && requires == "tar"
            )
        );
    }

    #[test]
    fn parse_multiple_types_fails() {
        use SourceParseError::VariantMultiple;
        let table = as_table(BAD_MULTIPLE_TYPES).unwrap();
        let source = Source::parse("src", &table);
        assert!(matches!(source, Err(VariantMultiple { source_name })
            if source_name == "src"
        ));
    }

    #[test]
    fn parse_missing_type_fails() {
        use SourceParseError::VariantUnknown;
        let table = as_table(BAD_MISSING_TYPE).unwrap();
        let source = Source::parse("src", &table);
        assert!(matches!(source, Err(VariantUnknown { source_name })
            if source_name == "src"
        ));
    }
}
