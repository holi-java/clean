use clean::{clean, IOResult};

#[tokio::main]
async fn main() -> IOResult<()> {
    let start_dir = std::env::args().nth(1);
    clean(start_dir.as_deref().unwrap_or(".")).await?;
    Ok(())
}
