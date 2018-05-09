use clap::{self, ArgMatches};
use sit_core::{Repository, Record};
use sit_core::cfg::{self, Configuration};
use chrono::prelude::*;
use std::process::exit;
use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use tempfile;
use atty;
use tempdir;
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
            let files = matches.values_of("FILES").unwrap_or(clap::Values::default());
            let types: Vec<_> = match matches.value_of("type") {
                Some(types) => types.split(",").collect(),
                None => vec![],
            };

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
                .map(|name| (name.clone(), ::std::fs::File::open(name).expect("can't open file")));

            let type_files = types.iter().map(|t|
                (format!(".type/{}", *t),
                 tempfile::tempfile_in(repo.path())
                     .expect(&format!("can't create a temporary file (.type/{})", t))));

            use std::io::{Write, Seek, SeekFrom};

            // .authors
            let authorship_files = if !matches.is_present("no-author") {
                let mut authors = tempfile::tempfile_in(repo.path()).expect("can't create a temporary file (.authors)");
                authors.write(format!("{}", config.author.clone().unwrap()).as_bytes()).expect("can't write to a tempoary file (.authors)");
                authors.seek(SeekFrom::Start(0)).expect("can't seek to the beginning of a temporary file (.authors)");
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
                item.new_record_in(record_path, files.chain(type_files).chain(authorship_files).chain(vec![(String::from(".timestamp"), f)].into_iter()), true)
            } else {
                item.new_record_in(record_path, files.chain(type_files).chain(authorship_files), true)
            }.expect("can't create a record");


            let signing = matches.is_present("sign") || config.signing.enabled;

            if signing {
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
    return 0;
}

