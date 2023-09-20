use clean::{clean, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let start_dir = std::env::args().nth(1);
    clean(start_dir.as_deref().unwrap_or(".")).await?;
    Ok(())
}
