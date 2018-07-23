use std::path::{Path, PathBuf};
use clap::ArgMatches;
use sit_core::{self, Repository};

use cli::{execute_cli, Error};

pub fn command<MI>(matches: &ArgMatches, repo: Repository<MI>, cwd: &Path) -> Result<i32, Error>
    where MI: sit_core::repository::ModuleIterator<PathBuf, sit_core::repository::Error> {
    let (subcommand, args) = matches.subcommand();
    let args = args.and_then(|args| args.values_of_lossy("")).unwrap_or(vec![]);
    return execute_cli::<_,_, &str, &str>(&repo, cwd, subcommand, args, None, false).map(|(code, _)| code);
}