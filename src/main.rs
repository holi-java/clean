use std::sync::Arc;

use clean::{clean, conf::Config, IOResult};

#[tokio::main(flavor = "multi_thread", worker_threads = 3)]
async fn main() -> IOResult<()> {
    let config = Config::default().await?;
    clean(".", Arc::new(config)).await?;
    Ok(())
}
