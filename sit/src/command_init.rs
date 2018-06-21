use std::path::Path;
use clap::ArgMatches;
use sit_core;

pub fn command<P1: AsRef<Path>, P2: AsRef<Path>>(init_matches: &ArgMatches, matches: &ArgMatches, working_dir: P1, default_repo: P2) -> i32 {
    let mut path = matches.value_of("repository").map(|r| working_dir.as_ref().join(r)).unwrap_or(default_repo.as_ref().into());
    if init_matches.is_present("no-dot-sit") && !matches.is_present("repository") {
        path.pop();
    }
    let path_str = path.to_str().unwrap();
    match sit_core::Repository::new(&path) {
        Ok(repo) => {
            if !init_matches.is_present("dont-populate") {
                repo.populate_default_files().expect("can't populate default files");
            }
            eprintln!("Repository {} initialized", path_str);
            return 0;
        }
        Err(sit_core::RepositoryError::AlreadyExists) => {
            eprintln!("Repository {} already exists", path_str);
            return 0;
        },
        Err(err) => {
            eprintln!("Error while initializing repository {}: {}", path_str, err);
            return 1;
        }
    }
}