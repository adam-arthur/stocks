use anyhow::{Result, ensure};

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    ensure!(
        args.len() <= 2,
        "usage: derive [input-directory [output-directory]]"
    );
    let input = std::path::Path::new(args.first().map_or("data/raw", String::as_str));
    let output = args
        .get(1)
        .map_or_else(|| std::path::PathBuf::from("data/derived"), Into::into);
    let count = derive::run(input, &output)?;
    println!("Derived {count} securities into {}", output.display());
    Ok(())
}
