#![feature(io_error_other)]
use std::{path::Path, pin::Pin, sync::Arc};

use conf::Config;
use futures::{future::try_join_all, Future};
pub type IOResult<T> = std::io::Result<T>;
use tokio::fs;
mod cmd;
pub mod conf;

#[async_recursion::async_recursion(?Send)]
pub async fn clean<P: AsRef<Path>>(entry: P, config: Arc<Config>) -> IOResult<bool> {
    let entry = entry.as_ref();
    let mut dir = fs::read_dir(entry).await?;
    let mut tasks: Vec<Pin<Box<dyn Future<Output = _>>>> = vec![];
    while let Some(path) = dir.next_entry().await?.map(|e| e.path()) {
        if let Some(plan) = config.parse(&path) {
            tasks.push(Box::pin(async move {
                let ok = plan.run(&entry).await;
                println!(
                    "clean: {}? {}",
                    entry.display(),
                    ok.as_ref()
                        .ok()
                        .filter(|ok| **ok)
                        .map(|_| "ok")
                        .unwrap_or("err")
                );
                ok
            }));
            break;
        }
        if path.is_dir() {
            let config = config.clone();
            tasks.push(Box::pin(
                /*BFS*/ async move { clean(path, config).await },
            ));
        }
    }
    drop(dir);
    return Ok(try_join_all(tasks).await?.into_iter().any(bool::from));
}
