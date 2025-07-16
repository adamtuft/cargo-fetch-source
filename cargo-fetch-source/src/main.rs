use fetch_source as fetch;

use fetch::Parse;

fn main() {
    let document = std::fs::read_to_string("Cargo.toml")
        .expect("Failed to read Cargo.toml")
        .parse::<toml::Table>()
        .unwrap();
    let sources_table = document
        .get("package")
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get("fetch-source"))
        .and_then(|v| v.as_table())
        .unwrap();
    let sources = fetch::Sources::try_parse(sources_table);
    println!("{sources:#?}");
}
