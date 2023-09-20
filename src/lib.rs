#![feature(io_error_other)]
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_recursion::async_recursion;
use conf::{Config, Plan};
use futures::future::try_join_all;
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

pub type IOResult<T> = std::io::Result<T>;
type Result = IOResult<bool>;

pub async fn clean<P>(entry: P) -> Result
where
    P: AsRef<Path>,
{
    clean_with_config(entry, Config::home().await?).await
}

pub async fn clean_with_config<P>(entry: P, config: Config) -> Result
where
    P: AsRef<Path>,
{
    let ncpus = num_cpus::get();
    let (tx, rx) = mpsc::channel::<Execution>(ncpus);

    let tasks = spawn((ncpus >> 1).max(1), Arc::new(Mutex::new(rx))).collect::<Vec<_>>();
    collect(entry, Arc::new(config), tx).await?;

    return try_join_all(tasks)
        .await?
        .into_iter()
        .try_fold(true, |status, result| result.map(|each| each || status));

    type ExecutionRecv = Arc<Mutex<Receiver<Execution<'static>>>>;
    fn spawn(n: usize, rx: ExecutionRecv) -> impl Iterator<Item = JoinHandle<Result>> {
        (0..n).map(move |_| {
            let rx = rx.clone();
            tokio::spawn(async move {
                let mut clean = false;
                while let Some(execution) = rx.lock().await.recv().await {
                    clean = execution.run().await? || clean;
                }
                Result::Ok(clean)
            })
        })
    }
}

#[async_recursion(?Send)]
async fn collect<P>(entry: P, config: Arc<Config>, tx: Sender<Execution<'static>>) -> IOResult<()>
where
    P: AsRef<Path>,
{
    macro_rules! try_unwrap {
        ($exp: expr) => {
            match $exp {
                Ok(value) => value,
                Err(err) => match err.kind() {
                    std::io::ErrorKind::NotFound => return Ok(()),
                    _ => return Err(err),
                },
            }
        };
    }
    let entry = entry.as_ref();
    let mut dir = try_unwrap!(fs::read_dir(entry).await);

    while let Some(current) = try_unwrap!(dir.next_entry().await).map(|e| e.path()) {
        if let Some(plan) = config.parse(&current) {
            let _ = tx.send(Execution(plan, entry.to_owned())).await;
        }
        if current.is_dir() {
            collect(current, config.clone(), tx.clone()).await?;
        }
    }
    return Ok(());
}

#[derive(Debug, Clone)]
struct Execution<'a>(Plan<'a>, PathBuf);

impl<'a> Execution<'a> {
    async fn run(&self) -> Result {
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
