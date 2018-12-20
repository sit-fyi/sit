use std::env;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use sit_core::{self, Repository, path::HasPath};
use which;
use derive_error::Error;

#[derive(Error, Debug)]
pub enum Error {
    WhichError,
    IoError(::std::io::Error),
}

impl From<which::Error> for Error {
    fn from(_err: which::Error) -> Self {
        Error::WhichError
    }
}

pub fn execute_cli<MI, E, K, V>(repo: &Repository<MI>, cwd: &Path, subcommand: &str, args: Vec<String>,
                       envs: E, capture_stdout: bool) -> Result<(i32, Vec<u8>), Error>
    where MI: sit_core::repository::ModuleIterator<PathBuf, sit_core::repository::Error>,
          E: IntoIterator<Item=(K, V)>, K: AsRef<OsStr>, V: AsRef<OsStr> {
    #[cfg(not(windows))]
    let path_sep = ":";
    #[cfg(windows)]
    let path_sep = ";";

    let cli_path = repo.path().join("cli");
    let mut path: String = cli_path.to_str().unwrap().into();
    for module_name in repo.module_iter().expect("can't iterate over modules") {
        path += path_sep;
        let module_name = module_name.expect("can't get module path");
        path += repo.modules_path().join(module_name).join("cli").to_str().unwrap().into();
    }

    path += path_sep;
    path += &env::var("PATH").unwrap();

    let cmd = format!("sit-{}", subcommand);
    let path = which::which_in(&cmd, Some(path), &cwd)
               .or_else(|err| match err.kind() {
                   #[cfg(unix)]
                   which::ErrorKind::CannotFindBinaryPath => {
                       let sh_cmd = format!("{}.sh", &cmd);
                       let sh_path = cli_path.join(&sh_cmd);
                       if sh_path.is_file() {
                           return Ok(sh_path)
                       }
                       for module_name in repo.module_iter().map_err(|_| {
                           let err: which::Error = which::ErrorKind::CannotGetCurrentDir.into();
                           err
                       })? {
                           if let Ok(name) = module_name {
                               let sh_path = repo.modules_path().join(name).join("cli").join(&sh_cmd);
                               if sh_path.is_file() {
                                   return Ok(sh_path)
                               }
                           }
                       }
                       Err(err)
                   },
                   _ => Err(err),
               })?;

    #[cfg(unix)]
    let exact = path.file_name().unwrap() == cmd.as_str();
    #[cfg(not(unix))]
    let exact = true;

    let mut command = if exact {
        ::std::process::Command::new(path)
    } else {
        let env = which::which("env")?;
        let mut cmd = ::std::process::Command::new(env);
        cmd.args(&["sh", path.to_str().unwrap()]);
        cmd
    };
    command.env("SIT_DIR", repo.path().to_str().unwrap());
    command.env("SIT", env::current_exe().unwrap_or("sit".into()).to_str().unwrap());
    command.args(args);
    if capture_stdout {
        command.stdout(::std::process::Stdio::piped());
    }
    command.envs(envs);
    let process = command.spawn()?;
    let result = process.wait_with_output().unwrap();
    Ok((result.status.code().unwrap(), result.stdout))
}
