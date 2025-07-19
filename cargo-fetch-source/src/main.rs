use anyhow::Context;

mod args;
mod fetch;
mod progress;

fn main() -> Result<(), anyhow::Error> {
    let args = args::parse()?;

    println!("{args:#?}");

    let document = std::fs::read_to_string(&args.manifest_file).context(format!(
        "Failed to read manifest file: {}",
        args.manifest_file.display()
    ))?;

    let artefacts = fetch_source::try_parse_toml(&document)
        .context("Failed to parse Cargo.toml")?
        .into_iter()
        .try_fold(Vec::new(), |artefacts, (name, source)| {
            fetch::fetch_serial(artefacts, name, source, &args.out_dir)
        })?;

    println!("\n🎉 Successfully fetched {} source(s)!", artefacts.len());
    Ok(())
}
