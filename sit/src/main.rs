extern crate sit_core;

extern crate chrono;
use chrono::prelude::*;
extern crate tempfile;
#[macro_use] extern crate clap;

use std::env;
use std::path::PathBuf;
use std::fs;
use std::process::exit;

use clap::{Arg, App, SubCommand};

use sit_core::{Issue, Record};
use sit_core::issue::IssueReduction;

extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

extern crate config;
mod cfg;

#[cfg(unix)]
extern crate xdg;

extern crate jmespath;

extern crate tini;

fn main() {
    #[cfg(unix)]
    let xdg_dir = xdg::BaseDirectories::with_prefix("sit").unwrap();

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
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .takes_value(true)
            .help("Config file (overrides default)"))
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
            .about("Lists issues")
            .arg(Arg::with_name("filter")
                     .long("filter")
                     .short("f")
                     .takes_value(true)
                     .default_value("type(@) == 'object'")
                     .help("Filter issues with a JMESPath query"))
            .arg(Arg::with_name("query")
                     .long("query")
                     .short("q")
                     .takes_value(true)
                     .default_value("join(' | ', [id, summary])")
                     .help("Render a result of a JMESPath query over the issue")))
        .subcommand(SubCommand::with_name("record")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Creates a new record")
            .arg(Arg::with_name("id")
                     .takes_value(true)
                     .required(true)
                     .help("Issue identifier"))
            .arg(Arg::with_name("type")
                .short("t")
                .long("type")
                .takes_value(true)
                .long_help("Comma-separated list of types to merge with supplied .type/TYPE files.")
                .help("Record type(s)"))
            .arg(Arg::with_name("no-timestamp")
                .long("no-timestamp")
                .help("By default, SIT will add a wall clock timestamp to all new. This option disables this behaviour"))
            .arg(Arg::with_name("no-author")
                .long("no-author")
                .help("By default, SIT will authorship information to all new records. This option disables this behaviour"))
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
                     .help("Issue identifier"))
            .arg(Arg::with_name("filter")
                     .long("filter")
                     .short("f")
                     .takes_value(true)
                     .default_value("type(@) == 'object'")
                     .help("Filter records with a JMESPath query"))
            .arg(Arg::with_name("query")
                     .long("query")
                     .short("q")
                     .takes_value(true)
                     .default_value("hash")
                     .help("Render a result of a JMESPath query over the record")))
        .subcommand(SubCommand::with_name("reduce")
            .about("Reduce issue records")
            .arg(Arg::with_name("id")
                     .takes_value(true)
                     .required(true)
                     .help("Issue identifier")))
        .get_matches();


    #[cfg(unix)]
    let default_config = PathBuf::from(xdg_dir.place_config_file("config.json").expect("can't create config directory"));
    #[cfg(windows)]
    let default_config = env::home_dir().expect("can't identify home directory").join("sit_config.json");

    let config_path = matches.value_of("config").unwrap_or(default_config.to_str().unwrap());

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name(config_path).required(false)).unwrap();

    let mut config: cfg::Configuration = settings.try_into().expect("can't load config");

    let working_dir = PathBuf::from(matches.value_of("working_directory").unwrap());
    let canonical_working_dir = fs::canonicalize(&working_dir).expect("can't canonicalize working directory");
    let dot_sit = working_dir.join(".sit");

    if config.author.is_none() {
        if let Some(author) = cfg::Author::from_gitconfig(canonical_working_dir.join(".git/config")) {
            config.author = Some(author);
        } else if let Some(author) = cfg::Author::from_gitconfig(env::home_dir().expect("can't identify home directory").join(".gitconfig")) {
            config.author = Some(author);
        } else {
            eprintln!("Authorship hasn't been configured. Update your {} config file\n\
            to include `author` object with `name` and `email` properties specified", config_path);
            exit(1);
        }
    }

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
        let repo = sit_core::Repository::find_in_or_above(".sit",&working_dir).expect("can't open repository");

        if let Some(matches) = matches.subcommand_matches("issue") {
            let issue = (if matches.value_of("id").is_none() {
                repo.new_issue()
            } else {
                repo.new_named_issue(matches.value_of("id").unwrap())
            }).expect("can't create an issue");
            println!("{}", issue.id());
        }

        if let Some(matches) = matches.subcommand_matches("issues") {
            let issues = repo.issue_iter().expect("can't list issues");

            let filter = jmespath::compile(matches.value_of("filter").unwrap()).expect("can't compile filter expression");
            let query = jmespath::compile(matches.value_of("query").unwrap()).expect("can't compile query expression");

            for issue in issues {
                let result = issue.reduce().expect("can't reduce issue");
                let json = sit_core::serde_json::to_string(&result).unwrap();
                let data = jmespath::Variable::from_json(&json).unwrap();
                let result = filter.search(&data).unwrap();
                if result.as_boolean().unwrap() {
                    let view = query.search(&data).unwrap();
                    if view.is_string() {
                        println!("{}", view.as_string().unwrap());
                    } else {
                        println!("{}", serde_json::to_string_pretty(&view).unwrap());
                    }
                }
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
                    let files = matches.values_of("FILES").unwrap_or(clap::Values::default());
                    let types: Vec<_> = matches.value_of("type").unwrap().split(",").collect();

                    if !files.clone().any(|f| f.starts_with(".type/")) && types.len() == 0 {
                        println!("At least one record type (.type/TYPE file) or `-t/--type` command line argument is required.");
                        exit(1);
                    }
                    let files = files.into_iter()
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
                            .map(|name| (name.clone(), ::std::fs::File::open(name).expect("can't open file")));

                    let type_files = types.iter().map(|t|
                                                          (format!(".type/{}", *t),
                                                           tempfile::tempfile_in(repo.path())
                                                               .expect(&format!("can't create a temporary file (.type/{})", t))));

                    use std::io::{Write, Seek, SeekFrom};

                    // .authors
                    let mut authors = tempfile::tempfile_in(repo.path()).expect("can't create a temporary file (.authors)");
                    authors.write(format!("{}", config.author.unwrap()).as_bytes()).expect("can't write to a tempoary file (.authors)");
                    authors.seek(SeekFrom::Start(0)).expect("can't seek to the beginning of a temporary file (.authors)");
                    let authorship_files = if !matches.is_present("no-author") {
                        vec![(".authors".into(), authors)].into_iter()
                    } else {
                        vec![].into_iter()
                    };

                    let record = if !matches.is_present("no-timestamp") {
                        let mut f = tempfile::tempfile_in(repo.path()).expect("can't create a temporary file (.timestamp)");
                        let utc: DateTime<Utc> = Utc::now();
                        f.write(format!("{:?}", utc).as_bytes()).expect("can't write to a temporary file (.timestamp)");
                        f.seek(SeekFrom::Start(0)).expect("can't seek to the beginning of a temporary file (.timestamp)");
                        issue.new_record(files.chain(type_files).chain(authorship_files).chain(vec![(String::from(".timestamp"), f)].into_iter()), true)
                    } else {
                        issue.new_record(files.chain(type_files).chain(authorship_files), true)
                    }.expect("can't create a record");
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

                    let filter = jmespath::compile(matches.value_of("filter").unwrap()).expect("can't compile filter expression");
                    let query = jmespath::compile(matches.value_of("query").unwrap()).expect("can't compile query expression");

                    for record in records {
                       for rec in record {
                           let json = sit_core::serde_json::to_string(&rec).unwrap();
                           let data = jmespath::Variable::from_json(&json).unwrap();
                           let result = filter.search(&data).unwrap();
                           if result.as_boolean().unwrap() {
                               let view = query.search(&data).unwrap();
                               if view.is_string() {
                                   println!("{}", view.as_string().unwrap());
                               } else {
                                   println!("{}", serde_json::to_string_pretty(&view).unwrap());
                               }
                           }
                       }
                    }
                }
            }
        }

        if let Some(matches) = matches.subcommand_matches("reduce") {
            let mut issues = repo.issue_iter().expect("can't list issues");
            let id = matches.value_of("id").unwrap();
            match issues.find(|i| i.id() == id) {
                None => {
                    println!("Issue {} not found", id);
                    exit(1);
                },
                Some(issue) => {
                    let result = issue.reduce().expect("can't reduce issue");
                    println!("{}", sit_core::serde_json::to_string_pretty(&result).unwrap());
                }
            }
        }

    }

}
