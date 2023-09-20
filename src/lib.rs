#![feature(io_error_other)]
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_recursion::async_recursion;
use conf::{Config, Plan};
use futures::future::try_join_all;
pub type IOResult<T> = std::io::Result<T>;
use tokio::{
    fs,
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
};
mod cmd;
pub mod conf;

pub async fn clean<P>(entry: P) -> IOResult<bool>
where
    P: AsRef<Path>,
{
    clean_with_config(entry, Config::home().await?).await
}

type ExecutionRecv = Arc<Mutex<Receiver<Execution<'static>>>>;

pub async fn clean_with_config<P>(entry: P, config: Config) -> IOResult<bool>
where
    P: AsRef<Path>,
{
    let (tx, rx) = mpsc::channel::<Execution>(5);

    let tasks = spawn(6, Arc::new(Mutex::new(rx))).collect::<Vec<_>>();
    collect(entry, Arc::new(config), tx).await?;

    return try_join_all(tasks)
        .await?
        .into_iter()
        .try_fold(true, |status, result| result.map(|each| each || status));

    fn spawn(n: usize, rx: ExecutionRecv) -> impl Iterator<Item = JoinHandle<IOResult<bool>>> {
        (0..n).map(move |_| {
            let rx = rx.clone();
            tokio::spawn(async move {
                let mut clean = false;
                while let Some(action) = rx.lock().await.recv().await {
                    clean = action.run().await? || clean;
                }
                IOResult::<_>::Ok(clean)
            })
        })
    }
}

#[async_recursion(?Send)]
async fn collect<P>(entry: P, config: Arc<Config>, tx: Sender<Execution<'static>>) -> IOResult<()>
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
            let _ = tx.send(Execution(plan, entry.to_owned())).await;
            discovered = true;
        }
        if current.is_dir() {
            sub_dirs.push(current);
        }
    }
    if !discovered {
        for dir in sub_dirs {
            collect(dir, config.clone(), tx.clone()).await?;
        }
    }
    return Ok(());
}

#[derive(Debug, Clone)]
struct Execution<'a>(Plan<'a>, PathBuf);

impl<'a> Execution<'a> {
    async fn run(&self) -> IOResult<bool> {
        let result = self.0.run(&self.1).await;
        println!(
            "clean: {} ? {}",
            self.1.display(),
            result
                .as_ref()
                .ok()
                .filter(|ok| **ok)
                .map(|_| "ok")
                .unwrap_or("error")
        );
        result
    }
}
