use clean::{clean, IOResult};

#[tokio::main(flavor = "multi_thread", worker_threads = 3)]
async fn main() -> IOResult<()> {
    let start_dir = std::env::args().skip(1).next();
    clean(start_dir.as_deref().unwrap_or(".")).await?;
    Ok(())
}
