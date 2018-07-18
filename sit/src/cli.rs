use std::env;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use sit_core::{self, Repository};
use which;

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

    let mut path: String = repo.path().join("cli").to_str().unwrap().into();
    for module_name in repo.module_iter().expect("can't iterate over modules") {
        path += path_sep;
        let module_name = module_name.expect("can't get module path");
        path += repo.modules_path().join(module_name).join("cli").to_str().unwrap().into();
    }

    path += path_sep;
    path += &env::var("PATH").unwrap();

    let path = which::which_in(format!("sit-{}", subcommand), Some(path), &cwd)?;
    let mut command = ::std::process::Command::new(path);
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