mod artefact;
mod error;
mod git;
mod process;
mod source;
#[cfg(feature = "tar")]
mod tar;

pub use error::Error;
pub use source::{Parse, Sources};

#[cfg(test)]
mod tests {
    use super::*;
    use super::source::fetch_source_blocking_helper_fn;
    use std::fs;

    #[test]
    #[cfg(feature = "tar")]
    fn print_sources_manually_extract() {
        match Sources::try_parse_toml(fs::read_to_string("Cargo.toml").unwrap()) {
            Ok(sources) => {
                println!("{sources:#?}");
            }
            Err(e) => {
                eprintln!("Error parsing sources: {e}");
            }
        }
    }

    // #[test]
    // fn test_fetch_sources_async() {
    //     let fetch_dir = std::path::PathBuf::from("test/test_fetch_sources_async");
    //     fs::create_dir_all(&fetch_dir).expect("Failed to create directory for fetching sources");
    //     let document = fs::read_to_string("Cargo.toml")
    //         .expect("Failed to read Cargo.toml")
    //         .parse::<toml::Table>()
    //         .unwrap();
    //     let sources = get_remote_sources_from_toml_table(&document).unwrap();
    //     let tok = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    //     let results = tok.block_on(async {
    //         let futures = sources
    //             .iter()
    //             .map(|(n, s)| fetch_source(n, s, fetch_dir.clone()));
    //         futures::future::join_all(futures).await
    //     });
    //     println!("Fetched sources: {results:#?}");
    // }

    #[test]
    fn test_fetch_sources_blocking() {
        let fetch_dir = std::path::PathBuf::from("test/test_fetch_sources_blocking");
        fs::create_dir_all(&fetch_dir).expect("Failed to create directory for fetching sources");
        let sources = Sources::try_parse_toml(fs::read_to_string("Cargo.toml").unwrap()).unwrap();
        let results = sources
            .iter()
            .map(|(n, s)| fetch_source_blocking_helper_fn(n, s, fetch_dir.clone()))
            .collect::<Vec<_>>();
        println!("Fetched sources: {results:#?}");
    }
}
