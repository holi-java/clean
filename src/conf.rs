use std::{collections::HashMap, ffi::OsString, io::Error, path::Path};

use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncRead, BufReader},
};

use crate::{cmd::Cmd, IOResult};

#[derive(Debug, Clone)]
pub enum Plan<'a> {
    Cmd(Cmd<'a>),
    RmDir(OsString),
}

impl<'a> Plan<'a> {
    #[cfg(test)]
    fn into_cmd(self) -> Option<Cmd<'a>> {
        match self {
            Plan::Cmd(cmd) => Some(cmd),
            Plan::RmDir(_) => None,
        }
    }

    fn filter<P: AsRef<Path>>(self, path: P) -> Option<Self> {
        match self {
            Plan::RmDir(_) if !path.as_ref().is_dir() => None,
            _ => Some(self),
        }
    }

    pub async fn run<P: AsRef<Path>>(&self, work_dir: P) -> IOResult<bool> {
        let work_dir = work_dir.as_ref();
        match self {
            Plan::Cmd(cmd) => cmd.run(work_dir).await.map(|status| status.success()),
            Plan::RmDir(dir) => tokio::fs::remove_dir_all(work_dir.join(dir))
                .await
                .map(|_| true),
        }
    }
}

type Registry = Box<dyn Fn() -> Plan<'static>>;

#[derive(Default)]
pub struct Config {
    registry: HashMap<String, Registry>,
}

unsafe impl Send for Config {}
unsafe impl Sync for Config {}

impl Config {
    pub async fn home() -> IOResult<Config> {
        match home::home_dir().map(|home| home.join(".cleanrc")) {
            Some(file) if file.is_file() => Self::load(File::open(file).await?).await,
            _ => Ok(Self::empty()),
        }
    }

    pub fn empty() -> Config {
        Default::default()
    }

    pub async fn load<T: AsyncRead + Unpin>(config: T) -> IOResult<Config> {
        let mut config = BufReader::new(config).lines();
        let mut registry = HashMap::<String, Registry>::new();
        while let Some(line) = config.next_line().await? {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(dir) = line.strip_suffix('/') {
                let dir = dir.to_string();
                registry.insert(
                    dir.to_string(),
                    Box::new(move || Plan::RmDir(dir.clone().into())),
                );
                continue;
            }

            let mut parts = line.splitn(2, '=').map(|s| s.trim());
            match (parts.next(), parts.next()) {
                (Some(file), Some(cmd)) if !file.is_empty() && !cmd.is_empty() => {
                    let cmd = cmd.split(' ').map(String::from).collect::<Vec<_>>();
                    registry.insert(
                        file.to_string(),
                        Box::new(move || {
                            Plan::Cmd(Cmd::new(
                                cmd[0].to_string(),
                                cmd[1..].iter().map(String::from),
                            ))
                        }),
                    );
                }
                _ => return help(),
            }
        }

        return Ok(Config { registry });

        fn help() -> IOResult<Config> {
            Err(Error::other(
                "\
# Config Examples:

# rm directory recursively
node_modules/

# run custom command
pom.xml = mvn -B clean
",
            ))
        }
    }

    pub fn parse<P: AsRef<Path>>(&self, path: P) -> Option<Plan<'static>> {
        let path = path.as_ref();
        let filename = path.file_name()?.to_str()?;
        match self.registry.get(filename) {
            Some(registry) => return registry().filter(path),
            _ => filename.parse().ok().map(Plan::Cmd),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::create_dir_all, path::Path, time::SystemTime};

    use crate::{
        cmd::Cmd,
        conf::{Config, Plan},
        IOResult,
    };

    #[tokio::test]
    async fn parse_empty_config() {
        let config = Config::empty();
        assert_eq!(
            config
                .parse("Cargo.toml")
                .unwrap()
                .into_cmd()
                .unwrap()
                .command,
            "cargo"
        );
    }

    #[tokio::test]
    async fn parse_dir_config() {
        let config = Config::load(b"node_modules/".as_ref()).await.unwrap();
        assert_eq!(
            config
                .parse("tests/Cargo.toml")
                .unwrap()
                .into_cmd()
                .unwrap()
                .command,
            "cargo"
        );
        assert!(matches!(
            config.parse("tests/data/node_modules").unwrap(),
            Plan::RmDir(dir) if dir == "node_modules"
        ));
    }

    #[tokio::test]
    async fn skip_comments() {
        let config = Config::load(b"#Node Dependencies Directory\n node_modules/".as_ref())
            .await
            .unwrap();
        assert!(matches!(
            config.parse("tests/data/node_modules").unwrap(),
            Plan::RmDir(dir) if dir == "node_modules"
        ));
    }

    #[tokio::test]
    async fn parse_trimmed_dir_config() {
        let config = Config::load(b" node_modules/ ".as_ref()).await.unwrap();

        assert!(matches!(
            config.parse("tests/data/node_modules").unwrap(),
            Plan::RmDir(dir) if dir == "node_modules"
        ));
    }

    #[tokio::test]
    async fn parse_trimmed_dir_contains_empty_lines() {
        let config = Config::load(b"node_modules/\r\n\r\ntarget/".as_ref())
            .await
            .unwrap();
        assert!(matches!(
            config.parse("tests/data/node_modules").unwrap(),
            Plan::RmDir(dir) if dir == "node_modules"
        ));
        assert!(matches!(
            config.parse("target").unwrap(),
            Plan::RmDir(dir) if dir == "target"
        ));
    }

    #[tokio::test]
    async fn parse_custom_cmd() {
        let config = Config::load(b"pom.xml = mvn -B clean".as_ref())
            .await
            .unwrap();
        let mvn = config.parse("pom.xml").unwrap().into_cmd().unwrap();
        assert_eq!(mvn.command, "mvn");
        assert_eq!(mvn.args, ["-B", "clean"]);
    }

    #[tokio::test]
    async fn fail_with_custom_empty_cmd() {
        let result = Config::load(b"pom.xml = ".as_ref()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn fail_with_empty_file_when_parse_custom_cmd() {
        let result = Config::load(b" = rm -rf".as_ref()).await;

        assert!(result.is_err());
    }

    #[test]
    fn rm_dir_plan_apply_dir_only() {
        assert!(Plan::RmDir("target".into()).filter("target").is_some());
        assert!(Plan::RmDir("target".into()).filter("Cargo.toml").is_none());
    }

    #[tokio::test]
    async fn run_rm_dir_plan() {
        let tmp = std::env::temp_dir();
        let test = tmp.join(format!(
            "test-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        create_dir_all(&test).unwrap();
        let _guard = RmDirGuard(&test);

        let rm = Plan::RmDir(test.file_name().unwrap().to_owned());
        let result: IOResult<bool> = rm.run(tmp).await;
        assert!(result.unwrap());
        assert!(!test.exists(), "dir should be removed");
    }

    #[tokio::test]
    async fn run_cmd_plan() {
        let tmp = std::env::temp_dir();
        let test = tmp.join(format!(
            "test-cmd-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let _guard = RmDirGuard(&test);
        create_dir_all(&test).unwrap();

        let rm = Plan::Cmd(Cmd::new(
            "rm",
            [
                "-d".to_string(),
                test.file_name().unwrap().to_string_lossy().to_string(),
            ],
        ));
        let result: IOResult<bool> = rm.run(tmp).await;
        assert!(result.unwrap());
        assert!(!test.exists(), "dir should be removed");
    }

    struct RmDirGuard<'a>(&'a Path);

    impl<'a> Drop for RmDirGuard<'a> {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0);
        }
    }
}