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

extern crate config;
use sit_core::cfg;

mod rebuild;
use rebuild::rebuild_repository;

#[cfg(unix)]
extern crate xdg;

extern crate jmespath;

extern crate tini;

extern crate fs_extra;
extern crate pbr;
extern crate tempdir;
extern crate glob;

extern crate rayon;
use rayon::prelude::*;

extern crate question;

use std::collections::HashMap;
fn get_named_expression<S: AsRef<str>>(name: S, repo: &sit_core::Repository,
                                       repo_path: S, exprs: &HashMap<String, String>) -> Option<String> {
    let path = repo.path().join(repo_path.as_ref()).join(name.as_ref());
    if path.is_file() {
        use std::fs::File;
        use std::io::Read;
        let mut f = File::open(path).unwrap();
        let mut result = String::new();
        f.read_to_string(&mut result).unwrap();
        Some(result)
    } else {
        exprs.get(name.as_ref()).map(String::clone)
    }
}

fn main() {
    exit(main_with_result());
}

fn main_with_result() -> i32 {
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
        .arg(Arg::with_name("repository")
            .short("r")
            .long("repository")
            .takes_value(true)
            .help("Point to a specific directory of SIT's repository"))
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
            .about("Initializes a new SIT repository in .sit")
            .arg(Arg::with_name("dont-populate")
                     .long("no-default-files")
                     .short("n")
                     .help("Don't populate repository with default files (such as reducers)")))
        .subcommand(SubCommand::with_name("populate-files")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("(Re)-populate default files in the repository (such as reducers)"))
        .subcommand(SubCommand::with_name("path")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Prints the path to the repository"))
        .subcommand(SubCommand::with_name("rebuild")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Rebuild a repository")
            .long_about("Useful for re-hashing all records while (optionally) \
            applying changes to them")
            .arg(Arg::with_name("SRC")
                     .takes_value(true)
                     .required(true)
                     .help("Source repository directory"))
            .arg(Arg::with_name("DEST")
                     .takes_value(true)
                     .required(true)
                     .help("Destination repository directory (must not exist)"))
            .arg(Arg::with_name("on-record")
                     .long("on-record")
                     .takes_value(true)
                     .long_help("Execute this command on every record before re-hashing it. \
                     The directory is passed as the first argument.")))
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
                     .conflicts_with("named-filter")
                     .long("filter")
                     .short("f")
                     .takes_value(true)
                     .help("Filter issues with a JMESPath query"))
            .arg(Arg::with_name("query")
                     .conflicts_with("named-query")
                     .long("query")
                     .short("q")
                     .takes_value(true)
                     .help("Render a result of a JMESPath query over the issue (defaults to `id`)"))
            .arg(Arg::with_name("named-filter")
                     .conflicts_with("filter")
                     .long("named-filter")
                     .short("F")
                     .takes_value(true)
                     .help("Filter issues with a named JMESPath query"))
            .arg(Arg::with_name("named-query")
                     .conflicts_with("query")
                     .long("named-query")
                     .short("Q")
                     .takes_value(true)
                     .help("Render a result of a named JMESPath query over the issue")))
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
            .arg(Arg::with_name("sign")
                .long("sign")
                .short("s")
                .help("Sign record with GnuPG (overrides config's signing.enabled)"))
            .arg(Arg::with_name("signing-key")
                .long("signing-key")
                .requires("sign")
                .takes_value(true)
                .help("Specify non-default signing key (overrides config's signing.key)"))
            .arg(Arg::with_name("gnupg")
                .long("gnupg")
                .requires("sign")
                .takes_value(true)
                .help("Specify gnupg command (`gpg` by default or overridden by config's signing.gnupg)"))
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
                     .conflicts_with("named-filter")
                     .long("filter")
                     .short("f")
                     .takes_value(true)
                     .help("Filter records with a JMESPath query"))
            .arg(Arg::with_name("query")
                     .conflicts_with("named-query")
                     .long("query")
                     .short("q")
                     .takes_value(true)
                     .help("Render a result of a JMESPath query over the record (defaults to `hash`)"))
            .arg(Arg::with_name("named-filter")
                     .conflicts_with("filter")
                     .long("named-filter")
                     .short("F")
                     .takes_value(true)
                     .help("Filter records with a named JMESPath query"))
            .arg(Arg::with_name("verify")
                     .short("v")
                     .long("verify")
                     .help("Verify record's signature (if present)"))
            .arg(Arg::with_name("gnupg")
                .long("gnupg")
                .requires("verify")
                .takes_value(true)
                .help("Specify gnupg command (`gpg` by default or overridden by config's signing.gnupg)"))
            .arg(Arg::with_name("named-query")
                     .conflicts_with("query")
                     .long("named-query")
                     .short("Q")
                     .takes_value(true)
                     .help("Render a result of a named JMESPath query over the record")))
        .subcommand(SubCommand::with_name("reduce")
            .about("Reduce issue records")
            .arg(Arg::with_name("id")
                     .takes_value(true)
                     .required(true)
                     .help("Issue identifier"))
            .arg(Arg::with_name("query")
                     .conflicts_with("named-query")
                     .long("query")
                     .short("q")
                     .takes_value(true)
                     .help("Render a result of a JMESPath query over the issue (defaults to `@`)"))
            .arg(Arg::with_name("named-query")
                     .conflicts_with("query")
                     .long("named-query")
                     .short("Q")
                     .takes_value(true)
                     .help("Render a result of a named JMESPath query over the issue")))
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
            println!("SIT needs your authorship identity to be configured\n");
            use question::{Question, Answer};
            let name = loop {
                match Question::new("What is your name?").ask() {
                    None => continue,
                    Some(Answer::RESPONSE(value)) => {
                        if value.trim() == "" {
                            continue;
                        } else {
                            break value;
                        }
                    },
                    Some(answer) => panic!("Invalid answer {:?}", answer),
                }
            };
            let email = match Question::new("What is your e-mail address?").clarification("optional").ask() {
                None => None,
                Some(Answer::RESPONSE(value)) => {
                    if value.trim() == "" {
                        None
                    } else {
                        Some(value)
                    }
                },
                Some(answer) => panic!("Invalid answer {:?}", answer),
            };
            config.author = Some(cfg::Author { name, email });
            let file = fs::File::create(config_path).expect("can't open config file for writing");
            serde_json::to_writer_pretty(file, &config).expect("can't write config");
        }
    }

    if let Some(init_matches) = matches.subcommand_matches("init") {
        let dot_sit_str = matches.value_of("repository").unwrap_or(dot_sit.to_str().unwrap());
        match sit_core::Repository::new(&dot_sit_str) {
            Ok(repo) => {
                if !init_matches.is_present("dont-populate") {
                    repo.populate_default_files().expect("can't populate default files");
                }
                eprintln!("Repository {} initialized", dot_sit_str);
            }
            Err(sit_core::RepositoryError::AlreadyExists) => {
                eprintln!("Repository {} already exists", dot_sit_str);
            },
            Err(err) => {
                eprintln!("Error while initializing repository {}: {}", dot_sit_str, err);
                return 1;
            }
        }
    } else if let Some(matches) = matches.subcommand_matches("rebuild") {
        rebuild_repository(matches.value_of("SRC").unwrap(),
                           matches.value_of("DEST").unwrap(),
                           matches.value_of("on-record"));
    } else {
        let repo = matches.value_of("repository").map(sit_core::Repository::open)
            .or_else(|| Some(sit_core::Repository::find_in_or_above(".sit",&working_dir)))
            .unwrap()
            .expect("can't open repository");

        if let Some(_) = matches.subcommand_matches("populate-files") {
            repo.populate_default_files().expect("can't populate default files");
        } else if let Some(_) = matches.subcommand_matches("path") {
            println!("{}", repo.path().to_str().unwrap());
        } else if let Some(matches) = matches.subcommand_matches("issue") {
            let issue = (if matches.value_of("id").is_none() {
                repo.new_issue()
            } else {
                repo.new_named_issue(matches.value_of("id").unwrap())
            }).expect("can't create an issue");
            println!("{}", issue.id());
        }

        if let Some(matches) = matches.subcommand_matches("issues") {
            let issues: Vec<_> = repo.issue_iter().expect("can't list issues").collect();

            let filter_expr = matches.value_of("named-filter")
                .and_then(|name|
                              get_named_expression(name, &repo, ".issues/filters", &config.issues.filters))
                .or_else(|| matches.value_of("filter").or_else(|| Some("`true`")).map(String::from))
                .unwrap();

            let filter_defined = matches.is_present("named-filter") || matches.is_present("filter");

            let query_expr = matches.value_of("named-query")
                .and_then(|name|
                              get_named_expression(name, &repo, ".issues/queries", &config.issues.queries))
                .or_else(|| matches.value_of("query").or_else(|| Some("id")).map(String::from))
                .unwrap();

            let filter = jmespath::compile(&filter_expr).expect("can't compile filter expression");
            let query = jmespath::compile(&query_expr).expect("can't compile query expression");

            let mut reducer = sit_core::reducers::duktape::DuktapeReducer::new(&repo).unwrap();
            let issues_with_reducers: Vec<_> =  issues.into_iter().map(|i| (i, reducer.clone())) .collect();
            issues_with_reducers.into_par_iter()
                .map(|(issue, mut reducer)| {
                    let result = issue.reduce_with_reducer(&mut reducer).expect("can't reduce issue");
                    let data = jmespath::Variable::from(serde_json::Value::Object(result));
                    let result = if filter_defined {
                        filter.search(&data).unwrap().as_boolean().unwrap()
                    } else {
                        true
                    };
                    if result {
                        let view = query.search(&data).unwrap();
                        if view.is_string() {
                            Some(view.as_string().unwrap().clone())
                        } else {
                            Some(serde_json::to_string_pretty(&view).unwrap())
                        }
                    } else {
                        None
                    }
                })
                .filter(Option::is_some).map(Option::unwrap)
                .for_each(|view| {
                    println!("{}", view);
                });
        }

        if let Some(matches) = matches.subcommand_matches("record") {
            let mut issues = repo.issue_iter().expect("can't list issues");
            let id = matches.value_of("id").unwrap();
            match issues.find(|i| i.id() == id) {
                None => {
                    eprintln!("Issue {} not found", id);
                    return 1;
                },
                Some(mut issue) => {
                    let files = matches.values_of("FILES").unwrap_or(clap::Values::default());
                    let types: Vec<_> = matches.value_of("type").unwrap().split(",").collect();

                    if !files.clone().any(|f| f.starts_with(".type/")) && types.len() == 0 {
                        println!("At least one record type (.type/TYPE file) or `-t/--type` command line argument is required.");
                        return 1;
                    }
                    let files = files.into_iter()
                            .map(move |name| {
                                let path = PathBuf::from(&name);
                                if !path.is_file() {
                                    eprintln!("{} does not exist or is not a regular file", path.to_str().unwrap());
                                    exit(1);
                                }
                                let abs_name = fs::canonicalize(path).expect("can't canonicalize path");
                                let cur_dir = fs::canonicalize(env::current_dir().expect("can't get current directory")).expect("can't canonicalize current directory");
                                match abs_name.strip_prefix(&cur_dir) {
                                    Err(_) => {
                                        eprintln!("Path {} is not relative to {} current directory", name, cur_dir.to_str().unwrap());
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
                    authors.write(format!("{}", config.author.clone().unwrap()).as_bytes()).expect("can't write to a tempoary file (.authors)");
                    authors.seek(SeekFrom::Start(0)).expect("can't seek to the beginning of a temporary file (.authors)");
                    let authorship_files = if !matches.is_present("no-author") {
                        vec![(".authors".into(), authors)].into_iter()
                    } else {
                        vec![].into_iter()
                    };

                    let tmp = tempdir::TempDir::new_in(repo.path(), "sit").unwrap();
                    let record_path = tmp.path();

                    let record = if !matches.is_present("no-timestamp") {
                        let mut f = tempfile::tempfile_in(repo.path()).expect("can't create a temporary file (.timestamp)");
                        let utc: DateTime<Utc> = Utc::now();
                        f.write(format!("{:?}", utc).as_bytes()).expect("can't write to a temporary file (.timestamp)");
                        f.seek(SeekFrom::Start(0)).expect("can't seek to the beginning of a temporary file (.timestamp)");
                        issue.new_record_in(record_path, files.chain(type_files).chain(authorship_files).chain(vec![(String::from(".timestamp"), f)].into_iter()), true)
                    } else {
                        issue.new_record_in(record_path, files.chain(type_files).chain(authorship_files), true)
                    }.expect("can't create a record");


                    let signing = matches.is_present("sign") || config.signing.enabled;

                    if signing {
                        use std::ffi::OsString;
                        let program = OsString::from(matches.value_of("gnupg").map(String::from)
                            .unwrap_or(match config.signing.gnupg {
                            Some(ref command) => command.clone(),
                            None => String::from("gpg"),
                        }));
                        let key = match matches.value_of("signing-key").map(String::from).or_else(|| config.signing.key.clone()) {
                            Some(key) => Some(OsString::from(key)),
                            None => None,
                        };
                        let mut command = ::std::process::Command::new(program);

                        command
                            .stdin(::std::process::Stdio::piped())
                            .stdout(::std::process::Stdio::piped())
                            .arg("--sign")
                            .arg("--armor")
                            .arg("--detach-sign")
                            .arg("-o")
                            .arg("-");

                        if key.is_some() {
                            let _ = command.arg("--default-key").arg(key.unwrap());
                        }

                        let mut child = command.spawn().expect("failed spawning gnupg");

                        {
                            let mut stdin = child.stdin.as_mut().expect("Failed to open stdin");
                            stdin.write_all(record.encoded_hash().as_bytes()).expect("Failed to write to stdin");
                        }

                        let output = child.wait_with_output().expect("failed to read stdout");

                        if !output.status.success() {
                            eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
                            return 1;
                        } else {
                            use sit_core::repository::DynamicallyHashable;
                            let dynamically_hashed_record = record.dynamically_hashed();
                            let mut file = fs::File::create(record.actual_path().join(".signature"))
                                .expect("can't open signature file");
                            file.write(&output.stdout).expect("can't write signature file");
                            drop(file);
                            let new_hash = dynamically_hashed_record.encoded_hash();
                            let mut new_path = record.path();
                            new_path.pop();
                            new_path.push(&new_hash);
                            fs::rename(record.actual_path(), new_path).expect("can't rename record");
                            println!("{}", new_hash);
                            return 0;
                        }

                    } else {
                        fs::rename(record.actual_path(), record.path()).expect("can't rename record");
                    }

                    println!("{}", record.encoded_hash());
                }
            }
        }

        if let Some(matches) = matches.subcommand_matches("records") {
            let mut issues = repo.issue_iter().expect("can't list issues");
            let id = matches.value_of("id").unwrap();
            match issues.find(|i| i.id() == id) {
                None => {
                    eprintln!("Issue {} not found", id);
                    return 1;
                },
                Some(issue) => {
                    let records = issue.record_iter().expect("can't lis records");

                    let filter_expr = matches.value_of("named-filter")
                        .and_then(|name|
                            get_named_expression(name, &repo, ".records/filters", &config.records.filters))
                        .or_else(|| matches.value_of("filter").or_else(|| Some("type(@) == 'object'")).map(String::from))
                        .unwrap();

                    let filter_defined = matches.is_present("named-filter") || matches.is_present("filter");

                    let query_expr = matches.value_of("named-query")
                        .and_then(|name|
                            get_named_expression(name, &repo, ".records/queries", &config.records.queries))
                        .or_else(|| matches.value_of("query").or_else(|| Some("hash")).map(String::from))
                        .unwrap();

                    let filter = jmespath::compile(&filter_expr).expect("can't compile filter expression");
                    let query = jmespath::compile(&query_expr).expect("can't compile query expression");

                    for record in records {
                       for rec in record {
                           // convert to JSON
                           let json = serde_json::to_string(&rec).unwrap();
                           // ...and back so that we can treat the record as a plain JSON
                           let mut json: serde_json::Value = serde_json::from_str(&json).unwrap();
                           if let serde_json::Value::Object(ref mut map) = json {
                               let verify = matches.is_present("verify") && rec.path().join(".signature").is_file();

                               if verify {
                                   use std::ffi::OsString;
                                   use std::io::Write;
                                   let program = OsString::from(matches.value_of("gnupg").map(String::from)
                                       .unwrap_or(match config.signing.gnupg {
                                           Some(ref command) => command.clone(),
                                           None => String::from("gpg"),
                                       }));
                                   let mut command = ::std::process::Command::new(program);

                                   command
                                       .stdin(::std::process::Stdio::piped())
                                       .stdout(::std::process::Stdio::piped())
                                       .stderr(::std::process::Stdio::piped())
                                       .arg("--verify")
                                       .arg(rec.path().join(".signature"))
                                       .arg("-");

                                   let mut child = command.spawn().expect("failed spawning gnupg");

                                   {
                                       use sit_core::repository::DynamicallyHashable;
                                       fn not_signature(val: &(String, fs::File)) -> bool {
                                           &val.0 != ".signature"
                                       }
                                       let filtered_record = rec.filtered(not_signature);
                                       let filtered_dynamic = filtered_record.dynamically_hashed();
                                       let mut stdin = child.stdin.as_mut().expect("Failed to open stdin");
                                       stdin.write_all(filtered_dynamic.encoded_hash().as_bytes()).expect("Failed to write to stdin");
                                   }

                                   let output = child.wait_with_output().expect("failed to read stdout");

                                   if !output.status.success() {
                                       let mut status = serde_json::Map::new();
                                       status.insert("success".into(), serde_json::Value::Bool(false));
                                       status.insert("output".into(), serde_json::Value::String(String::from_utf8_lossy(&output.stderr).into()));
                                       map.insert("verification".into(), serde_json::Value::Object(status));
                                   } else {
                                       let mut status = serde_json::Map::new();
                                       status.insert("success".into(), serde_json::Value::Bool(true));
                                       status.insert("output".into(), serde_json::Value::String(String::from_utf8_lossy(&output.stderr).into()));
                                       map.insert("verification".into(), serde_json::Value::Object(status));
                                   }

                               }

                           }

                           let data = jmespath::Variable::from(json);
                           let result = if filter_defined {
                               filter.search(&data).unwrap().as_boolean().unwrap()
                           } else {
                               true
                           };
                           if result {
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
                    eprintln!("Issue {} not found", id);
                    return 1;
                },
                Some(issue) => {
                    let query_expr = matches.value_of("named-query")
                        .and_then(|name|
                            get_named_expression(name, &repo, ".issues/queries", &config.issues.queries))
                        .or_else(|| matches.value_of("query").or_else(|| Some("@")).map(String::from))
                        .unwrap();

                    let query = jmespath::compile(&query_expr).expect("can't compile query expression");

                    let mut reducer = sit_core::reducers::duktape::DuktapeReducer::new(&repo).unwrap();
                    let result = issue.reduce_with_reducer(&mut reducer).expect("can't reduce issue");
                    let data = jmespath::Variable::from(serde_json::Value::Object(result));
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

    return 0;

}
