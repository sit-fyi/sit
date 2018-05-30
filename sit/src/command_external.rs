use std::env;
use std::path::Path;
use clap::ArgMatches;
use sit_core::Repository;
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

pub fn command(matches: &ArgMatches, repo: Repository, cwd: &Path) -> Result<i32, Error> {
    let (subcommand, args) = matches.subcommand();

    #[cfg(not(windows))]
    let path_sep = ":";
    #[cfg(windows)]
    let path_sep = ";";

    let mut path: String = repo.path().join("cli").to_str().unwrap().into();
    for module_name in repo.module_iter().expect("can't iterate over modules") {
        path += path_sep;
        path += repo.modules_path().join(module_name).join("cli").to_str().unwrap().into();
    }

    path += path_sep;
    path += &env::var("PATH").unwrap();

    let path = which::which_in(format!("sit-{}", subcommand), Some(path), &cwd)?;
    let mut command = ::std::process::Command::new(path);
    command.env("SIT_DIR", repo.path().to_str().unwrap());
    command.env("SIT", env::current_exe().unwrap_or("sit".into()).to_str().unwrap());
    if let Some(args) = args {
        command.args(args.values_of_lossy("").unwrap_or(vec![]));
    }
    let mut process = command.spawn()?;
    let result = process.wait().unwrap();
    Ok(result.code().unwrap())
}