#![feature(io_error_other)]
use std::{path::Path, pin::Pin, sync::Arc};

use async_recursion::async_recursion;
use conf::Config;
use futures::{future::try_join_all, Future};
pub type IOResult<T> = std::io::Result<T>;
use tokio::fs;
mod cmd;
pub mod conf;

type Task<T> = Pin<Box<dyn Future<Output = IOResult<T>>>>;

pub async fn clean<P>(entry: P) -> IOResult<bool>
where
    P: AsRef<Path>,
{
    clean_with_config(entry, Config::home().await?).await
}

pub async fn clean_with_config<P>(entry: P, config: Config) -> IOResult<bool>
where
    P: AsRef<Path>,
{
    let config = Arc::new(config);
    let mut plans = vec![];
    collect(entry, config, &mut plans).await?;
    Ok(try_join_all(plans).await?.into_iter().any(bool::from))
}

#[async_recursion(?Send)]
async fn collect<P>(entry: P, config: Arc<Config>, tasks: &mut Vec<Task<bool>>) -> IOResult<()>
where
    P: AsRef<Path>,
{
    let entry = entry.as_ref();
    let mut dir = fs::read_dir(entry).await?;
    let mut sub_dirs = vec![];
    let mut discovered = false;

    while let Some(current) = dir.next_entry().await?.map(|e| e.path()) {
        if let Some(plan) = config.parse(&current) {
            let entry = entry.to_owned();
            println!("found: {} ...", current.display());
            tasks.push(Box::pin(async move {
                let result = plan.run(&entry).await;
                println!(
                    "{}? {}",
                    entry.display(),
                    result
                        .as_ref()
                        .ok()
                        .filter(|ok| **ok)
                        .map(|_| "ok")
                        .unwrap_or("err")
                );
                return result;
            }));
            discovered = true;
        }
        if current.is_dir() {
            sub_dirs.push(current);
        }
    }
    if !discovered {
        for dir in sub_dirs {
            collect(dir, config.clone(), tasks).await?;
        }
    }
    return Ok(());
}
