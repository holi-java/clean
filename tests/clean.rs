use std::{io, path::Path};

use clean_rs::{clean_with_config, conf::Config, Result};
use tokio::fs;

#[path = "../src/test.rs"]
mod test;

use test::RmDirGuard;

#[tokio::test]
async fn clean_dir() {
    let start = std::env::temp_dir().join("test");
    let _guard = RmDirGuard(&start);
    let _ = fs::remove_dir_all(&start).await;
    fs::create_dir_all(&start).await.unwrap();
    copy("tests/data", &start).await.unwrap();

    let root = start.join("data");
    let to_removed = root.join("target");
    assert!(to_removed.exists());

    assert!(clean(&start).await.unwrap());
    assert!(!to_removed.exists());
    assert!(root.join("Cargo.toml").exists());
}

#[tokio::test]
async fn clean_dir_recursively() {
    let start = std::env::temp_dir().join("a/b/c");
    let _guard = RmDirGuard(start.join("../.."));
    let _ = fs::remove_dir_all(&start).await;
    fs::create_dir_all(&start).await.unwrap();
    copy("tests/data", &start).await.unwrap();

    let root = start.join("data");
    let to_removed = root.join("target");
    assert!(to_removed.exists());

    assert!(clean(&start.join("../..")).await.unwrap());
    assert!(!to_removed.exists());
    assert!(root.join("Cargo.toml").exists());
}

#[tokio::test]
async fn clean_all_generated_dirs() {
    let start = std::env::temp_dir().join("multiple");
    let _guard = RmDirGuard(&start);
    let _ = fs::remove_dir_all(&start).await;
    fs::create_dir_all(start.join("a")).await.unwrap();
    copy("tests/data", start.join("a")).await.unwrap();
    fs::create_dir_all(start.join("b")).await.unwrap();
    copy("tests/data", start.join("b")).await.unwrap();

    let a = start.join("a/data/target");
    let b = start.join("b/data/target");
    assert!(a.exists());
    assert!(b.exists());

    assert!(clean(&start).await.unwrap());
    assert!(!a.exists());
    assert!(!b.exists());
}

#[tokio::test]
async fn reports_error_if_entry_path_does_not_exists() {
    let err = clean("absent").await.unwrap_err();

    assert_eq!(err.to_string(), "Directory not found: absent");
}

#[tokio::test]
async fn reports_error_if_entry_path_is_not_directory() {
    let err = clean("Cargo.toml").await.unwrap_err();

    assert_eq!(err.to_string(), "Cargo.toml is not a directory");
}

#[async_recursion::async_recursion(?Send)]
async fn copy<S: AsRef<Path>, D: AsRef<Path>>(src: S, dest: D) -> io::Result<()> {
    let (src, dest) = (src.as_ref(), dest.as_ref());
    let mut dir = fs::read_dir(src).await?;
    let dest = dest.join(src.file_name().unwrap());
    fs::create_dir(&dest).await?;
    while let Some(file) = dir.next_entry().await? {
        if file.path().is_dir() {
            copy(file.path(), &dest).await?;
            continue;
        }
        fs::copy(file.path(), dest.join(file.file_name())).await?;
    }
    Ok(())
}

pub async fn clean<P>(entry: P) -> Result<bool>
where
    P: AsRef<Path>,
{
    clean_with_config(entry, Config::empty()).await
}
