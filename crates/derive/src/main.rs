use anyhow::{Result, ensure};

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    ensure!(
        args.len() <= 2,
        "usage: derive [input-directory [output-directory]] | derive --yield-stats [derived-directory]"
    );
    if args.first().is_some_and(|arg| arg == "--yield-stats") {
        let output = std::path::Path::new(args.get(1).map_or("data/derived", String::as_str));
        println!(
            "Updated yield statistics for {} securities",
            derive::refresh_yield_stats(output)?
        );
        return Ok(());
    }
    let input = std::path::Path::new(args.first().map_or("data/raw", String::as_str));
    let output = args
        .get(1)
        .map_or_else(|| std::path::PathBuf::from("data/derived"), Into::into);
    let count = derive::run(input, &output)?;
    println!("Derived {count} securities into {}", output.display());
    Ok(())
}
