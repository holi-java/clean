use std::{path::Path, pin::Pin};

use cmd::Cmd;
use futures::{future::try_join_all, Future};
pub type IOResult<T> = std::io::Result<T>;
use tokio::fs;
mod cmd;

#[async_recursion::async_recursion(?Send)]
pub async fn clean<P: AsRef<Path>>(entry: P) -> IOResult<bool> {
    let entry = entry.as_ref();
    let mut dir = fs::read_dir(entry).await?;
    let mut tasks: Vec<Pin<Box<dyn Future<Output = _>>>> = vec![];
    while let Some(file) = dir.next_entry().await? {
        let path = file.path();
        if path.is_file() {
            if let Some(Ok(cmd)) = file.file_name().to_str().map(|file| file.parse::<Cmd>()) {
                tasks.push(Box::pin(async move {
                    let status = cmd.run(&entry).await?;
                    println!(
                        "clean: {}? {}",
                        entry.display(),
                        status
                            .code()
                            .filter(|&i| i == 0)
                            .map(|_| "ok")
                            .unwrap_or("error")
                    );
                    Ok(status.success())
                }));
            }
        } else if path.is_dir() {
            tasks.push(Box::pin(/*BFS*/ async move { clean(path).await }));
        }
    }
    drop(dir);
    return Ok(try_join_all(tasks).await?.into_iter().any(bool::from));
}
