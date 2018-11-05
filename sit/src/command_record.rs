use atty;
use chrono::prelude::*;
use clap::{self, ArgMatches};
use dunce;
use serde_json;
use crate::cfg::{self, Configuration};
use sit_core::{
    record::{BoxedOrderedFiles, OrderedFiles, RecordOwningContainer},
    Record, Repository
};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Cursor, ErrorKind, Write};
use std::path::{Path, PathBuf};
use walkdir::{self as walk, WalkDir};
use itertools::Itertools;

#[cfg(feature = "deprecated-items")]
pub const FILES_ARG: &str = "[Item ID (DEPRECATED)] FILES";
#[cfg(not(feature = "deprecated-items"))]
pub const FILES_ARG: &str = "FILES";

#[cfg(feature = "deprecated-items")]
pub const FILES_ARG_HELP: &str = "Optional item identifier followed by a collection of files or folders the record will be built from";
#[cfg(not(feature = "deprecated-items"))]
pub const FILES_ARG_HELP: &str = "Collection of files or folders the record will be built from";

fn record_files(
    matches: &ArgMatches, offset: usize,
    utc: DateTime<Utc>,
    config: &Configuration,
) -> Result<BoxedOrderedFiles<'static>, io::Error> {
    let files = matches
        .values_of(FILES_ARG)
        .unwrap_or(clap::Values::default());

    let files: OrderedFiles<_> = files
        .dropping(offset)
        .into_iter()
        .map(|name| {
            let path = PathBuf::from(&name);
            if path.is_file() {
                Ok(vec![(OsString::from(name), path)])
            } else if path.is_dir() {
                let entries = WalkDir::new(path)
                    .into_iter()
                    .filter_map(|f| {
                        if let Ok(f) = f {
                            if f.metadata().expect("error reading metadata").is_file() {
                                Some(f)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .map(|entry: walk::DirEntry| {
                        (entry.file_name().to_owned(), entry.path().to_owned())
                    })
                    .collect();

                Ok(entries)
            } else {
                Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "{} does not exist or is not a regular file",
                        path.to_str().unwrap()
                    ),
                ))
            }
        })
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .flat_map(|files| {
            files.iter().map(|(name, path)| {
                let abs_name = dunce::canonicalize(path).expect("can't canonicalize path");
                let cur_dir = dunce::canonicalize(
                    env::current_dir().expect("can't get current directory"),
                ).expect("can't canonicalize current directory");
                match abs_name.strip_prefix(&cur_dir) {
                    Err(_) => Err(io::Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "Path {:?} is not relative to {} current directory",
                            name,
                            cur_dir.to_str().unwrap()
                        ),
                    )),
                    Ok(path) => Ok(String::from(path.to_str().unwrap())),
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .map(|name| (name.clone(), fs::File::open(name).expect("can't open file")))
        .into();

    let types: Vec<_> = match matches.value_of("type") {
        Some(types) => types.split(",").collect(),
        None => vec![],
    };

    let type_files: OrderedFiles<_> = types
        .iter()
        .map(|t| (format!(".type/{}", *t), &[][..]))
        .into();

    // .authors
    let authorship_files: Option<OrderedFiles<(String, _)>> = if !matches.is_present("no-aux") && !matches.is_present("no-author") {
        let authors = format!("{}", config.author.clone().unwrap());
        Some(vec![(String::from(".authors"), Cursor::new(authors))].into())
    } else {
        None
    };

    let timestamp: Option<OrderedFiles<(String, _)>> = if !matches.is_present("no-aux") && !matches.is_present("no-timestamp") {
        let timestamp = format!("{:?}", utc);
        Some(vec![(String::from(".timestamp"), Cursor::new(timestamp))].into())
    } else {
        None
    };

    Ok(files + type_files + authorship_files + timestamp)
}

pub fn command<P: AsRef<Path>, P1: AsRef<Path>, MI>(matches: &ArgMatches, repo: &Repository<MI>, mut config: Configuration, working_directory: P, config_path: P1) -> i32 {
    if !matches.is_present("no-aux") && !matches.is_present("no-author") && config.author.is_none() {
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
                let file =
                    fs::File::create(config_path).expect("can't open config file for writing");
                serde_json::to_writer_pretty(file, &config).expect("can't write config");
            } else {
                eprintln!("SIT needs your authorship identity to be configured (supported sources: sit, git), or re-run this command in a terminal\n");
                return 1;
            }
        }
    }

    #[cfg(feature = "deprecated-items")]
    let offset = {
        let item = matches.value_of(FILES_ARG)
            // file with such a name doesn't exist
            .and_then(|maybe_id|
                if Path::new(maybe_id).exists() {
                    None
                } else {
                    Some(maybe_id)
                })
            // item with such a name exists
            .and_then(|id| repo.item(id));

        let first_is_file = matches.value_of(FILES_ARG)
            .and_then(|name|
                if Path::new(name).exists() {
                    Some(name)
                } else {
                    None
                });

        let val = matches.value_of(FILES_ARG).unwrap_or("<unknown>");

        if matches.value_of(FILES_ARG).is_some() && item.is_none() && first_is_file.is_none() {
            eprintln!("Item or file {} not found", val);
            return 1;
        }
        if item.is_some() && first_is_file.is_some() {
            eprintln!("Ambiguity detected: {} is both a file and an item identifier", val);
            return 1;
        }
        if item.is_some() {
            1
        } else {
            0
        }
    };
    #[cfg(not(feature = "deprecated-items"))]
    let offset = 0;

    let utc: DateTime<Utc> = Utc::now();

    let signing = matches.is_present("sign") || config.signing.enabled;

    let files = record_files(matches, offset, utc, &config).expect("failed collecting files");

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
            let files = record_files(matches, offset, utc, &config).expect("failed collecting files");
            let signature_file: OrderedFiles<(String, _)> = vec![(String::from(".signature"), Cursor::new(output.stdout))].into();
            files + signature_file
        }

    } else {
        files
    };

    let record = if offset == 1 { // item
        let item = matches.value_of(FILES_ARG)
            .and_then(|id| repo.item(id))
            .unwrap();

        item.new_record(files, true).expect("can't create a record")
    } else { // repo
        repo.new_record(files, true).expect("can't create a record")
    };

    println!("{}", record.encoded_hash());

    return 0;
}

