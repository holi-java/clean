#![allow(clippy::needless_return)]
#![doc = include_str!("../README.md")]

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_recursion::async_recursion;
use conf::{Config, Plan};
use futures::future::{try_join_all, TryJoinAll};
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
mod error;
pub use error::Error;

pub(crate) type IOResult<T> = std::io::Result<T>;
pub type Result<T> = std::result::Result<T, Error>;

pub async fn clean<P>(entry: P) -> Result<bool>
where
    P: AsRef<Path>,
{
    clean_with_config(entry, Config::home().await?).await
}

pub async fn clean_with_config<P>(entry: P, config: Config) -> Result<bool>
where
    P: AsRef<Path>,
{
    let ncpus = num_cpus::get();
    let (tx, rx) = mpsc::channel::<Execution>(ncpus);

    let tasks = spawn((ncpus >> 1).max(1), Arc::new(Mutex::new(rx)));
    collect(entry, Arc::new(config), tx).await?;

    return tasks
        .await?
        .into_iter()
        .try_fold(true, |status, result| result.map(|each| each || status));

    type ExecutionRecv = Arc<Mutex<Receiver<Execution<'static>>>>;
    fn spawn(n: usize, rx: ExecutionRecv) -> TryJoinAll<JoinHandle<Result<bool>>> {
        try_join_all((0..n).map(move |_| {
            let rx = rx.clone();
            tokio::spawn(async move {
                let mut clean = false;
                while let Some(execution) = rx.lock().await.recv().await {
                    clean = execution.run().await? || clean;
                }
                Result::Ok(clean)
            })
        }))
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
    async fn run(&self) -> Result<bool> {
        use termcolor::{
            Buffer, BufferWriter, Color, ColorChoice, ColorSpec, HyperlinkSpec, WriteColor,
        };
        let result = self.0.run(&self.1).await;
        write(&result, &self.1)?;

        return result;

        fn write(result: &Result<bool>, path: &Path) -> Result<()> {
            use std::io::{stdout, IsTerminal};
            let out = BufferWriter::stdout(match stdout().is_terminal() {
                true => ColorChoice::Always,
                _ => ColorChoice::Never,
            });
            Ok(out.print(&try_concat(tag(path, &out), colorized(result, &out))?)?)
        }

        fn try_concat(head: IOResult<Buffer>, tail: IOResult<Buffer>) -> IOResult<Buffer> {
            let mut buf = Buffer::ansi();
            buf.write_all(head?.as_slice())?;
            buf.write_all(tail?.as_slice())?;
            buf.write_all(b"\n")?;
            Ok(buf)
        }

        fn tag(path: &Path, out: &BufferWriter) -> IOResult<Buffer> {
            let mut buf = out.buffer();
            write!(buf, "clean: ")?;
            use path_absolutize::Absolutize;
            let url = format!("file://{}", path.absolutize()?.display());
            #[cfg(target_os = "windows")]
            let url = url.replace('\\', "/");
            buf.set_hyperlink(&HyperlinkSpec::open(url.as_bytes()))?;
            write!(buf, "{}", path.display())?;
            buf.set_hyperlink(&HyperlinkSpec::close())?;
            write!(buf, "? ")?;
            Ok(buf)
        }

        fn colorized(result: &Result<bool>, out: &BufferWriter) -> IOResult<Buffer> {
            let mut buf = out.buffer();
            let (fg, text) = if let Ok(true) = result {
                (Color::Green, "ok")
            } else {
                (Color::Red, "error")
            };
            let mut spec = ColorSpec::new();
            buf.set_color(spec.set_fg(Some(fg)))?;
            write!(buf, "{}", text)?;
            spec.clear();
            buf.set_color(&spec)?;
            Ok(buf)
        }
    }
}
