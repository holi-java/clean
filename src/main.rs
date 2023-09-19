use clean::{clean, IOResult};

#[tokio::main(flavor = "multi_thread", worker_threads = 3)]
async fn main() -> IOResult<()> {
    clean(".").await?;
    Ok(())
}
