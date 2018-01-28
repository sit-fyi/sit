extern crate sit_core;

#[macro_use] extern crate clap;

use std::env;
use std::path::PathBuf;
use std::fs;
use std::process::exit;

use clap::{Arg, App, SubCommand};

use sit_core::{Issue, Record};

fn main() {
    let cwd = env::current_dir().expect("can't get current working directory");
    let matches = App::new("SIT")
        .version(crate_version!())
        .about(crate_description!())
        .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
        .arg(Arg::with_name("working_directory")
            .short("d")
            .default_value(cwd.to_str().unwrap())
            .help("Working directory"))
        .arg(Arg::with_name("verbosity")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .subcommand(SubCommand::with_name("init")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Initializes a new SIT repository in .sit"))
        .subcommand(SubCommand::with_name("issue")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Creates a new issue")
            .arg(Arg::with_name("id")
                     .long("id")
                     .takes_value(true)
                     .required(false)
                     .help("Specify issue identifier, otherwise generate automatically")))
        .subcommand(SubCommand::with_name("issues")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Lists issues"))
        .subcommand(SubCommand::with_name("record")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Creates a new record")
            .arg(Arg::with_name("id")
                     .takes_value(true)
                     .required(true)
                     .help("Issue identifier"))
            .arg(Arg::with_name("FILES")
                     .multiple(true)
                     .takes_value(true)
                     .help("Collection of files the record will be built from")))
        .subcommand(SubCommand::with_name("records")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Lists records")
             .arg(Arg::with_name("id")
                     .takes_value(true)
                     .required(true)
                     .help("Issue identifier")))
        .get_matches();

    let working_dir = PathBuf::from(matches.value_of("working_directory").unwrap());
    let canonical_working_dir = fs::canonicalize(&working_dir).expect("can't canonicalize working directory");
    let mut dot_sit = working_dir;
    dot_sit.push(".sit");

    if let Some(_matches) = matches.subcommand_matches("init") {
        let dot_sit_str = dot_sit.to_str().unwrap();
        match sit_core::Repository::new(&dot_sit) {
            Ok(_repo) => {
                eprintln!("Repository {} initialized", dot_sit_str);
            }
            Err(sit_core::RepositoryError::AlreadyExists) => {
                eprintln!("Repository {} already exists", dot_sit_str);
            },
            Err(err) => {
                eprintln!("Error while initializing repository {}: {}", dot_sit_str, err);
                exit(1);
            }
        }
    } else {
        // iterate the working directory up to find an existing repository
        loop {
            if !dot_sit.is_dir() {
                // get out of .sit
                dot_sit.pop();
                // if can't pop anymore, we're at the root of the filesystem
                if !dot_sit.pop() {
                     eprintln!("Can't find a repository");
                    exit(1);
                }
                // try assuming current path + .sit
                dot_sit.push(".sit");
            } else {
                break;
            }
        }

        let repo = sit_core::Repository::open(&dot_sit).expect("can't open repository");

        if let Some(matches) = matches.subcommand_matches("issue") {
            let issue = (if matches.value_of("id").is_none() {
                repo.new_issue()
            } else {
                repo.new_named_issue(matches.value_of("id").unwrap())
            }).expect("can't create an issue");
            println!("{}", issue.id());
        }

        if let Some(_matches) = matches.subcommand_matches("issues") {
            let issues = repo.issue_iter().expect("can't list issues");
            for issue in issues {
                println!("{}", issue.id());
            }
        }

        if let Some(matches) = matches.subcommand_matches("record") {
            let mut issues = repo.issue_iter().expect("can't list issues");
            let id = matches.value_of("id").unwrap();
            match issues.find(|i| i.id() == id) {
                None => {
                    println!("Issue {} not found", id);
                    exit(1);
                },
                Some(issue) => {
                    let record = issue.new_record(
                        matches.values_of("FILES").unwrap().into_iter()
                            .map(move |name| {
                                let path = PathBuf::from(&name);
                                if !path.is_file() {
                                    eprintln!("{} does not exist or is not a regular file", path.to_str().unwrap());
                                    exit(1);
                                }
                                let abs_name = fs::canonicalize(path).expect("can't canonicalize path");
                                match abs_name.strip_prefix(&canonical_working_dir) {
                                    Err(_) => {
                                        eprintln!("Path {} is not relative to {} working directory", name, canonical_working_dir.to_str().unwrap());
                                        exit(1);
                                    },
                                    Ok(path) => String::from(path.to_str().unwrap()),
                                }
                            })
                            .map(|name| (name.clone(), ::std::fs::File::open(name).expect("can't open file"))), true).expect("can't create a record");
                    println!("{}", record.encoded_hash());
                }
            }
        }

        if let Some(matches) = matches.subcommand_matches("records") {
            let mut issues = repo.issue_iter().expect("can't list issues");
            let id = matches.value_of("id").unwrap();
            match issues.find(|i| i.id() == id) {
                None => {
                    println!("Issue {} not found", id);
                    exit(1);
                },
                Some(issue) => {
                    let records = issue.record_iter().expect("can't lis records");
                    for record in records {
                       for rec in record {
                           println!("{}", rec.encoded_hash());
                       }
                    }
                }
            }
        }

    }

}
