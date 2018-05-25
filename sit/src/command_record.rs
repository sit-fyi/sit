use clap::{self, ArgMatches};
use sit_core::{Repository, Record, Item, record::{OrderedFiles, BoxedOrderedFiles}};
use sit_core::cfg::{self, Configuration};
use chrono::prelude::*;
use std::process::exit;
use std::io::{self, Cursor, Write};
use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use atty;
use serde_json;

pub fn command<P: AsRef<Path>, P1: AsRef<Path>>(matches: &ArgMatches, repo: &Repository, mut config: Configuration, working_directory: P, config_path: P1) -> i32 {
    if !matches.is_present("no-author") && config.author.is_none() {
        if let Some(author) = cfg::Author::from_gitconfig(working_directory.as_ref().join(".git").join("config")) {
            config.author = Some(author);
        } else {
            if atty::is(atty::Stream::Stdin) {
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
            } else {
                eprintln!("SIT needs your authorship identity to be configured (supported sources: sit, git), or re-run this command in a terminal\n");
                return 1;
            }
        }
    }

    let id = matches.value_of("id").unwrap();
    match repo.item(id) {
        None => {
            eprintln!("Item {} not found", id);
            return 1;
        },
        Some(item) => {
            fn record_files(matches: &ArgMatches, utc: DateTime<Utc>, config: &Configuration) -> Result<BoxedOrderedFiles<'static>, io::Error> {
                let files = matches.values_of("FILES").unwrap_or(clap::Values::default());
                let files: OrderedFiles<_> = files.into_iter()
                    .map(move |name| {
                        let path = PathBuf::from(&name);
                        if !path.is_file() {
                            eprintln!("{} does not exist or is not a regular file", path.to_str().unwrap());
                            exit(1);
                        }
                        let abs_name = ::dunce::canonicalize(path).expect("can't canonicalize path");
                        let cur_dir = ::dunce::canonicalize(env::current_dir().expect("can't get current directory")).expect("can't canonicalize current directory");
                        match abs_name.strip_prefix(&cur_dir) {
                            Err(_) => {
                                eprintln!("Path {} is not relative to {} current directory", name, cur_dir.to_str().unwrap());
                                exit(1);
                            },
                            Ok(path) => String::from(path.to_str().unwrap()),
                        }
                    })
                    .map(|name| (name.clone(), ::std::fs::File::open(name).expect("can't open file"))).into();

                let types: Vec<_> = match matches.value_of("type") {
                    Some(types) => types.split(",").collect(),
                    None => vec![],
                };

                let type_files: OrderedFiles<_> = types.iter().map(|t|
                    (format!(".type/{}", *t),&[][..])).into();

                // .authors
                let authorship_files: Option<OrderedFiles<(String, _)>> = if !matches.is_present("no-author") {
                    let authors = format!("{}", config.author.clone().unwrap());
                    Some(vec![(String::from(".authors"), Cursor::new(authors))].into())
                } else {
                    None
                };

                let timestamp: Option<OrderedFiles<(String, _)>>= if !matches.is_present("no-timestamp") {
                    let timestamp = format!("{:?}", utc);
                    Some(vec![(String::from(".timestamp"), Cursor::new(timestamp))].into())
                } else {
                    None
                };

                Ok(files + type_files + authorship_files + timestamp)

            }

            let utc: DateTime<Utc> = Utc::now();


            let signing = matches.is_present("sign") || config.signing.enabled;

            let files = record_files(matches, utc, &config).expect("failed collecting files");

            let files = if signing {
                use std::ffi::OsString;
                let program = super::gnupg(matches, &config).expect("can't find GnuPG");
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
                    let mut hasher = repo.config().hashing_algorithm().hasher();
                    files.hash(&mut *hasher).expect("failed hashing files");
                    let hash = hasher.result_box();
                    let encoded_hash = repo.config().encoding().encode(&hash);
                    stdin.write_all(encoded_hash.as_bytes()).expect("Failed to write to stdin");
                }

                let output = child.wait_with_output().expect("failed to read stdout");

                if !output.status.success() {
                    eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
                    return 1;
                } else {
                    let files = record_files(matches, utc, &config).expect("failed collecting files");
                    let signature_file: OrderedFiles<(String, _)> = vec![(String::from(".signature"), Cursor::new(output.stdout))].into();
                    files + signature_file
                }

            } else {
                files
            };

            let record = item.new_record(files, true).expect("can't create a record");

            println!("{}", record.encoded_hash());
        }
    }
    return 0;
}

