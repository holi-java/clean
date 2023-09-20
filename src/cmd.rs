use std::{borrow::Cow, path::Path, process::ExitStatus, str::FromStr};

use tokio::process::{Child, Command};

use crate::IOResult;

#[derive(Debug, Clone)]
pub struct Cmd<'a> {
    pub command: Cow<'a, str>,
    pub args: Vec<Cow<'a, str>>,
}

impl<'a> Cmd<'a> {
    pub fn new<T, A>(command: T, args: A) -> Self
    where
        Cow<'a, str>: From<T>,
        A: IntoIterator,
        Cow<'a, str>: From<A::Item>,
    {
        Cmd {
            command: Cow::from(command),
            args: args.into_iter().map(Cow::from).collect(),
        }
    }

    pub async fn run<P>(&self, work_dir: P) -> IOResult<ExitStatus>
    where
        P: AsRef<Path>,
    {
        self.execute(work_dir).await?.wait().await
    }

    #[inline]
    async fn execute<P: AsRef<Path>>(&self, work_dir: P) -> IOResult<Child> {
        let mut cmd = Command::new(self.command.as_ref());
        let cmd = cmd
            .args(self.args.iter().map(|arg| arg.as_ref()))
            .current_dir(work_dir.as_ref());

        let cmd = {
            use std::process::Stdio;
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped())
        };

        cmd.spawn()
    }
}

impl<'a> FromStr for Cmd<'a> {
    type Err = String;

    fn from_str(command: &str) -> Result<Self, Self::Err> {
        if let Some(command) = command.strip_prefix('!') {
            let mut parts = command.split(' ').map(String::from);
            return Ok(Cmd::new(parts.next().unwrap(), parts));
        }

        macro_rules! resolve {
            ($(($cmd:ident, $($tt:tt)*)),*) => {
                match command {
                    $(stringify!($($tt)*) => Ok(Cmd::new(stringify!($cmd), ["clean"])),)*
                    _ => Err(format!("command can not be resolved: `{command}`")),
                }
            };
        }

        resolve!(
            (cargo, Cargo.toml),
            (go, go.mod),
            (mvn, pom.xml),
            (gradle, build.gradle)
        )
    }
}

#[cfg(test)]
mod tests {

    use crate::cmd::Cmd;

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn run() {
        let pwd = Cmd::new("pwd", [] as [&str; 0]);
        assert!(pwd.run(".").await.unwrap().success());
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn echo_working_directory() {
        let pwd = Cmd::new("pwd", [] as [&str; 0]);
        let out = pwd
            .execute("/home")
            .await
            .unwrap()
            .wait_with_output()
            .await
            .unwrap();
        assert!(out.status.success());
        assert_eq!(String::from_utf8(out.stdout.clone()).unwrap(), "/home\n");
    }

    #[test]
    fn builtin_commands() {
        let tests = [
            ("Cargo.toml", "cargo"),
            ("go.mod", "go"),
            ("pom.xml", "mvn"),
            ("build.gradle", "gradle"),
        ];
        for (file, expected) in tests {
            let cmd = file.parse::<Cmd>().unwrap();
            assert_eq!(cmd.command, expected);
            assert_eq!(cmd.args, ["clean"]);
        }
    }

    #[test]
    fn custom_commands() {
        let rm = "!rm -rf .".parse::<Cmd>().unwrap();
        assert_eq!(rm.command, "rm");
        assert_eq!(rm.args, ["-rf", "."]);
    }
}
