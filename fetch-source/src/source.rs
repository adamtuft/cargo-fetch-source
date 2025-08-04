//! Core types for intereacting with sources declared in `Cargo.toml`.

use super::error::FetchError;
use super::git::Git;
#[cfg(feature = "tar")]
use super::tar::Tar;

/// The name of a source
pub type SourceName = String;

/// Inner error type for source parsing errors
#[derive(Debug, thiserror::Error)]
enum SourceParseErrorInner {
    /// An unknown source variant was encountered.
    #[error("expected a valid source type for source '{source_name}': expected one of: {known}", known = SOURCE_VARIANTS.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))]
    VariantUnknown {
        /// The name of the source whose variant wasn't recognised
        source_name: SourceName,
    },

    /// A source has multiple variants given.
    #[error("multiple source types for source '{source_name}': expected exactly one of: {known}", known = SOURCE_VARIANTS.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))]
    VariantMultiple {
        /// The name of the source with multiple variants
        source_name: SourceName,
    },

    /// A source has a variant which depends on a disabled feature.
    #[error("source '{source_name}' has type '{variant}' but needs disabled feature '{requires}'")]
    VariantDisabled {
        /// The name of the source
        source_name: SourceName,
        /// The source type
        variant: String,
        /// The disabled feature
        requires: String,
    },

    /// A toml value was expected to be a table.
    #[error("expected value '{name}' to be a toml table")]
    ValueNotTable {
        /// The key for the value which was expected to be a table
        name: String,
    },

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

/// Errors encountered when parsing sources from `Cargo.toml`
/// 
/// This uses a boxed newtype pattern to reduce the size of Result types
/// containing this error, addressing clippy::result_large_err warnings.
#[derive(Debug)]
pub struct SourceParseError(Box<SourceParseErrorInner>);

impl SourceParseError {
    /// Create a new VariantUnknown error
    pub fn variant_unknown(source_name: SourceName) -> Self {
        Self(Box::new(SourceParseErrorInner::VariantUnknown { source_name }))
    }

    /// Create a new VariantMultiple error
    pub fn variant_multiple(source_name: SourceName) -> Self {
        Self(Box::new(SourceParseErrorInner::VariantMultiple { source_name }))
    }

    /// Create a new VariantDisabled error
    pub fn variant_disabled(source_name: SourceName, variant: String, requires: String) -> Self {
        Self(Box::new(SourceParseErrorInner::VariantDisabled { source_name, variant, requires }))
    }

    /// Create a new ValueNotTable error
    pub fn value_not_table(name: String) -> Self {
        Self(Box::new(SourceParseErrorInner::ValueNotTable { name }))
    }

    /// Create a new SourceTableNotFound error
    pub fn source_table_not_found() -> Self {
        Self(Box::new(SourceParseErrorInner::SourceTableNotFound))
    }

    /// Check if this error is a VariantUnknown error (for testing)
    #[cfg(test)]
    pub fn is_variant_unknown(&self) -> bool {
        matches!(&*self.0, SourceParseErrorInner::VariantUnknown { .. })
    }

    /// Check if this error is a VariantMultiple error (for testing)
    #[cfg(test)]
    pub fn is_variant_multiple(&self) -> bool {
        matches!(&*self.0, SourceParseErrorInner::VariantMultiple { .. })
    }

    /// Check if this error is a VariantDisabled error (for testing)
    #[cfg(test)]
    pub fn is_variant_disabled(&self) -> bool {
        matches!(&*self.0, SourceParseErrorInner::VariantDisabled { .. })
    }

    /// Check if this error is a ValueNotTable error (for testing)
    #[cfg(test)]
    pub fn is_value_not_table(&self) -> bool {
        matches!(&*self.0, SourceParseErrorInner::ValueNotTable { .. })
    }

    /// Check if this error is a SourceTableNotFound error (for testing)
    #[cfg(test)]
    pub fn is_source_table_not_found(&self) -> bool {
        matches!(&*self.0, SourceParseErrorInner::SourceTableNotFound)
    }

    /// Get the source name from the error if applicable (for testing)
    #[cfg(test)]
    pub fn source_name(&self) -> Option<&str> {
        match &*self.0 {
            SourceParseErrorInner::VariantUnknown { source_name } => Some(source_name),
            SourceParseErrorInner::VariantMultiple { source_name } => Some(source_name),
            SourceParseErrorInner::VariantDisabled { source_name, .. } => Some(source_name),
            _ => None,
        }
    }

    /// Get the variant and requires fields from VariantDisabled errors (for testing)
    #[cfg(test)]
    pub fn variant_info(&self) -> Option<(&str, &str)> {
        match &*self.0 {
            SourceParseErrorInner::VariantDisabled { variant, requires, .. } => Some((variant, requires)),
            _ => None,
        }
    }

    /// Check if this error is a TomlInvalid error (for testing)
    #[cfg(test)]
    pub fn is_toml_invalid(&self) -> bool {
        matches!(&*self.0, SourceParseErrorInner::TomlInvalid(_))
    }

    /// Get the name field from ValueNotTable errors (for testing)
    #[cfg(test)]
    pub fn table_name(&self) -> Option<&str> {
        match &*self.0 {
            SourceParseErrorInner::ValueNotTable { name } => Some(name),
            _ => None,
        }
    }
}

impl std::fmt::Display for SourceParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for SourceParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<toml::de::Error> for SourceParseError {
    fn from(err: toml::de::Error) -> Self {
        Self(Box::new(SourceParseErrorInner::TomlInvalid(err)))
    }
}

impl From<serde_json::Error> for SourceParseError {
    fn from(err: serde_json::Error) -> Self {
        Self(Box::new(SourceParseErrorInner::JsonInvalid(err)))
    }
}

/// Represents the result of a fetch operation
pub type FetchResult<T> = Result<T, crate::FetchError>;

/// Represents a source that has been fetched from a remote location.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Artefact {
    // This is a combination of the fetched artefact and the source it was fetched from.
    // Note that the name associated with a source *must not* be stored in the cache. This avoids
    // using one name for a source but then unexpectedly returning another.
    /// The upstream source
    source: Source,
    /// The local copy
    path: std::path::PathBuf,
}

impl AsRef<std::path::Path> for Artefact {
    fn as_ref(&self) -> &std::path::Path {
        &self.path
    }
}

impl AsRef<Source> for Artefact {
    fn as_ref(&self) -> &Source {
        &self.source
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
    /// A remote tar archive
    Tar(Tar),
    #[serde(rename = "git")]
    /// A remote git repo
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
    pub fn fetch<P: AsRef<std::path::Path>>(self, dir: P) -> FetchResult<Artefact> {
        let dest = dir.as_ref();
        let result = match self {
            #[cfg(feature = "tar")]
            Source::Tar(ref tar) => tar.fetch(dest),
            Source::Git(ref git) => git.fetch(dest),
        };
        match result {
            Ok(path) => Ok(Artefact { source: self, path }),
            Err(err) => Err(FetchError::new(err, self)),
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
                    return Err(SourceParseError::variant_multiple(name.to_string()));
                }
                if !variant.is_enabled() {
                    return Err(SourceParseError::variant_disabled(
                        name.to_string(),
                        variant.to_string(),
                        variant.feature().unwrap_or("?").to_string(),
                    ));
                }
                detected_variant = Some(variant);
            }
        }
        detected_variant.ok_or_else(|| SourceParseError::variant_unknown(name.to_string()))
    }

    /// Parse a TOML table into a `Source` instance. Exactly one key in the table must identify
    /// a valid, enabled source type, otherwise an error is returned.
    pub fn parse<S: ToString>(name: S, source: toml::Table) -> Result<Self, SourceParseError> {
        Self::enforce_one_valid_variant(name, &source)?;
        Ok(toml::Value::Table(source).try_into::<Self>()?)
    }
}

/// Represents the contents of the `package.metadata.fetch-source` table in a `Cargo.toml` file.
pub type SourcesTable = std::collections::HashMap<SourceName, Source>;

/// Parse a `package.metadata.fetch-source` table into a [`SourcesTable`](crate::source::SourcesTable) map
pub fn try_parse(table: &toml::Table) -> Result<SourcesTable, SourceParseError> {
    table
        .iter()
        .map(|(k, v)| match v.as_table() {
            Some(t) => Source::parse(k, t.to_owned()).map(|s| (k.to_owned(), s)),
            None => Err(SourceParseError::value_not_table(k.to_owned())),
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
        .ok_or_else(SourceParseError::source_table_not_found)?;
    try_parse(sources_table)
}

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
            matches!(source, Err(err) if err.is_variant_disabled() && 
                err.variant_info() == Some(("tar", "tar")))
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
        assert!(matches!(source, Err(err) if err.is_variant_multiple() && 
            err.source_name() == Some("src")));
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
        assert!(matches!(source, Err(err) if err.is_variant_unknown() && 
            err.source_name() == Some("src")));
    }
}

#[cfg(test)]
mod test_parsing_sources_table_failure_modes {
    use super::*;

    #[test]
    fn parse_invalid_toml_str_fails() {
        let document = "this is not a valid toml document :( uh-oh!";
        let result = try_parse_toml(document);
        assert!(matches!(result, Err(err) if err.is_toml_invalid()));
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
        assert!(matches!(try_parse_toml(document), Err(err) if err.is_source_table_not_found()));
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
            Err(err) if err.is_value_not_table() && err.table_name() == Some("not-a-table")
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
            Err(err) if err.is_variant_disabled() && 
                err.source_name() == Some("bar") && 
                err.variant_info() == Some(("tar", "tar"))
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
            Err(err) if err.is_variant_multiple() && err.source_name() == Some("bar")
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
            Err(err) if err.is_variant_unknown() && err.source_name() == Some("bar")
        ));
    }
}
