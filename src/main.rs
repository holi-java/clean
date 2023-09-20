use std::process::exit;

use clean_rs::{clean, Result};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{}", err);
        exit(1);
    }
}

async fn run() -> Result<()> {
    let start_dir = std::env::args().nth(1);
    clean(start_dir.as_deref().unwrap_or(".")).await?;
    Ok(())
}
