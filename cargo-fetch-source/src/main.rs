use anyhow::Context;

use fetch_source::Artefact;

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

    let handles = fetch_source::try_parse_toml(&document)
        .context("Failed to parse Cargo.toml")?
        .into_iter()
        .fold(Vec::new(), |handles, (name, source)| {
            fetch::fetch_parallel(handles, name, source, &args.out_dir)
        });

    let mut success = 0usize;
    for join in handles.into_iter().map(|h| h.join()) {
        match join {
            Ok(Ok(Artefact::Git(git))) => {
                println!("✅ 🔗 Cloned repository into {}", git.local.display());
                success += 1;
            }
            Ok(Ok(Artefact::Tar(tar))) => {
                println!("✅ 📦 Extracted {} into {}", tar.url, &args.out_dir.display());
                success += 1;
            }
            Ok(Err(fetch_error)) => {
                eprintln!("❌ Failed to fetch source: {fetch_error}");
            }
            Err(thread_error) => {
                eprintln!("❌ Thread panicked: {thread_error:?}");
            }
        }
    }

    println!("\n🎉 Successfully fetched {success} source(s)!");

    Ok(())
}
