use clap::ArgMatches;
use sit_core::{Repository, Item, repository::{Error}};
use std::io::ErrorKind as IoErrorKind;

pub fn command<MI>(matches: &ArgMatches, repo: &Repository<MI>) -> i32 {
    let named = matches.value_of("id");
    let item = if named.is_none() {
        repo.new_item()
    } else {
        repo.new_named_item(named.clone().unwrap())
    };
    match item {
        Ok(item) => {
            println!("{}", item.id());
            return 0;
        },
        Err(Error::IoError(err)) => {
            if err.kind() == IoErrorKind::AlreadyExists {
                eprintln!("Item {} already exists", named.unwrap());
                return 1;
            } else {
                panic!("can't create an item: {:?}", err)
            }
        },
        Err(err) =>
            panic!("can't create an item: {:?}", err),
    }
}

