extern crate sit_core;

extern crate chrono;
extern crate tempfile;
#[macro_use]
extern crate clap;

use std::env;
use std::path::PathBuf;
use std::process::exit;

use clap::{App, Arg, ArgMatches, SubCommand};

extern crate serde;
extern crate serde_json;
extern crate walkdir;
extern crate yaml_rust;

extern crate config;

mod cfg;
mod rebuild;
use crate::rebuild::rebuild_repository;
mod command_config;
mod command_args;
mod command_init;
mod command_item;
mod command_record;
mod command_items;
mod command_reduce;
mod command_records;
mod command_external;
mod command_jmespath;
mod command_integrity;
#[cfg(feature="web")]
mod command_web;
mod authorship;

mod cli;

extern crate jmespath;

extern crate fs_extra;
extern crate pbr;
extern crate tempdir;
extern crate glob;

extern crate atty;

extern crate rayon;

extern crate question;

extern crate dunce;

extern crate which;
use which::which;

extern crate thread_local;

#[macro_use] extern crate derive_error;
extern crate directories;
extern crate itertools;

#[cfg(feature="web")]
#[macro_use]
extern crate rouille;
#[cfg(feature="web")]
extern crate mime_guess;
#[cfg(feature="web")]
extern crate digest;
#[cfg(feature="web")]
extern crate blake2;
#[cfg(feature="web")]
extern crate hex;
#[cfg(feature="web")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "git")] extern crate git2;
#[macro_use] extern crate serde_derive;

use std::collections::HashMap;
pub fn get_named_expression<S: AsRef<str>, MI>(name: S, repo: &sit_core::Repository<MI>,
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

use std::ffi::OsString;
pub fn gnupg(matches: &ArgMatches, config: &cfg::Configuration) -> Result<OsString, which::Error> {
    let program = OsString::from(matches.value_of("gnupg").map(String::from)
        .unwrap_or(match config.signing.gnupg {
            Some(ref command) => command.clone(),
            None => which("gpg2").or_else(|_| which("gpg"))?.to_str().unwrap().into(),
        }));
    Ok(program)
}

mod module_iter;
use crate::module_iter::ScriptModule;

use sit_core::path::HasPath;

trait ConditionalChain : Sized {
    fn conditionally<F: Fn(Self) -> Self>(self, cond: bool, f: F) -> Self {
        if cond {
            f(self)
        } else {
            self
        }
    }
}

impl<T> ConditionalChain for T where T : Sized {
}

fn main() {
    exit(main_with_result(true));
}

fn main_with_result(allow_external_subcommands: bool) -> i32 {
    let cwd = env::current_dir().expect("can't get current working directory");

    let mut app = App::new("SIT")
        .version(crate_version!())
        .about(crate_description!())
        .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
        .arg(Arg::with_name("working_directory")
            .short("d")
            .takes_value(true)
            .help("Working directory (defaults to current directory)"))
        .arg(Arg::with_name("repository")
            .short("r")
            .long("repository")
            .takes_value(true)
            .env("SIT_DIR")
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
        .arg(Arg::with_name("disable-integrity-check")
                 .short("i")
                 .long("disable-integrity-check")
                 .help("Disables record integrity check (mostly for performance reasons)"))
        .subcommand(SubCommand::with_name("integrity")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Checks the integrity of record hashes and lists invalid records"))
        .subcommand(SubCommand::with_name("upgrade")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Upgrades the repository"))
        .subcommand(SubCommand::with_name("init")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Initializes a new SIT repository in .sit")
            .arg(Arg::with_name("no-dot-sit")
                .long("no-dot-sit")
                .short("u")
                .help("Initialize the repo in working directory or <repository> if overriden with -r"))
            .arg(Arg::with_name("dont-populate")
                     .long("no-default-files")
                     .short("n")
                     .help("Don't populate repository with default files (such as reducers)")))
        .subcommand(SubCommand::with_name("populate-files")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("(Re)-populate default files in the repository (such as reducers)"))
        .subcommand(SubCommand::with_name("path")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .arg(Arg::with_name("record")
                .long("record")
                .short("r")
                .takes_value(true)
                .help("Path to the record"))
            .about("Prints the path to the repository and its individual components"))
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
        .conditionally(cfg!(feature = "deprecated-items"), |app|
        app.subcommand(SubCommand::with_name("item")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Creates a new item")
            .arg(Arg::with_name("id")
                     .long("id")
                     .takes_value(true)
                     .required(false)
                     .help("Specify item identifier, otherwise generate automatically"))))
        .conditionally(cfg!(feature = "deprecated-items"), |app|
        app.subcommand(SubCommand::with_name("items")
               .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
               .about("Lists items (DEPRECATED)")
               .arg(Arg::with_name("filter")
                   .conflicts_with("named-filter")
                   .long("filter")
                   .short("f")
                   .takes_value(true)
                   .help("Filter items with a JMESPath query"))
               .arg(Arg::with_name("query")
                   .conflicts_with("named-query")
                   .long("query")
                   .short("q")
                   .takes_value(true)
                   .help("Render a result of a JMESPath query over the item (defaults to `id`)"))
               .arg(Arg::with_name("named-filter")
                   .conflicts_with("filter")
                   .long("named-filter")
                   .short("F")
                   .takes_value(true)
                   .help("Filter items with a named JMESPath query"))
               .arg(Arg::with_name("named-query")
                   .conflicts_with("query")
                   .long("named-query")
                   .short("Q")
                   .takes_value(true)
                   .help("Render a result of a named JMESPath query over the item"))))
        .subcommand(SubCommand::with_name("record")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Creates a new record")
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
            .arg(Arg::with_name("no-aux")
                .long("no-aux")
                .short("n")
                .help("By default, SIT will attempt to add auxiliary information. This option disables this behaviour"))
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
            .arg(Arg::with_name(command_record::FILES_ARG)
                     .multiple(true)
                     .takes_value(true)
                     .help(command_record::FILES_ARG_HELP)))
        .subcommand(SubCommand::with_name("records")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Lists records")
            .conditionally(cfg!(feature = "deprecated-items"), |app|
            app.arg(Arg::with_name("id")
                     .takes_value(true)
                     .help("Item identifier (DEPRECATED)")))
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
            .about("Reduce records")
            .conditionally(cfg!(feature = "deprecated-items"), |app|
            app.arg(Arg::with_name("id")
                     .takes_value(true)
                     .help("Item identifier (DEPRECATED)")))
            .arg(Arg::with_name("reducer")
                 .short("r")
                 .long("reducer")
                 .takes_value(true)
                 .multiple(true)
                 .help("Specifies custom reducers to be used (instead of default ones in reducers/)"))
            .arg(Arg::with_name("root")
                 .long("root")
                 .short("R")
                 .takes_value(true)
                 .multiple(true)
                 .help("Specifies fixed roots to begin the reduction from"))
            .arg(Arg::with_name("format")
                 .short("f")
                 .long("format")
                 .default_value("json")
                 .possible_values(&["json"])
                 .help("State's format"))
            .arg(Arg::with_name("state")
                 .short("s")
                 .long("state")
                 .takes_value(true)
                 .validator(|v| serde_json::from_str(&v).map_err(|e| format!("JSON parsing error: {}", e))
                                .and_then(|v: serde_json::Value| if v.is_object() { Ok(()) } else { Err(format!("Expected JSON object, got {}", v)) }))
                 .help("Initial state"))
            .arg(Arg::with_name("query")
                     .conflicts_with("named-query")
                     .long("query")
                     .short("q")
                     .takes_value(true)
                     .help("Render a result of a JMESPath query (defaults to `@`)"))
            .arg(Arg::with_name("named-query")
                     .conflicts_with("query")
                     .long("named-query")
                     .short("Q")
                     .takes_value(true)
                     .help("Render a result of a named JMESPath query")))
        .subcommand(SubCommand::with_name("config")
            .about("Prints configuration file")
            .arg(Arg::with_name("kind")
                     .possible_values(&["user", "repository"])
                     .default_value("user")
                     .help("Configuration kind"))
            .arg(Arg::with_name("query")
                     .long("query")
                     .short("q")
                     .takes_value(true)
                     .help("JMESPath query (none by default)")))
        .subcommand(SubCommand::with_name("modules")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Prints out resolved modules"))
        .subcommand(SubCommand::with_name("jmespath")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .arg(Arg::with_name("expr")
                .required(true)
                .takes_value(true)
                .help("JMESPath expression"))
            .arg(Arg::with_name("pretty")
                .short("p")
                .long("pretty")
                .help("Prettify JSON output"))
            .about("Evaluates a JMESPath expression over a JSON read from stdin"))
        .subcommand(SubCommand::with_name("args")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .arg(Arg::with_name("help")
                .long("help")
                .short("h")
                .help("Prints help"))
            .arg(Arg::with_name("ARGS")
                .last(true)
                .multiple(true)
                .conflicts_with("help")
                .help("Arguments to parse"))
            .about("Parses arguments against a specification given on stdin"))
        .conditionally(cfg!(feature = "web"), |app|
        app.subcommand(SubCommand::with_name("web")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("HTTP API server providing access to the repository")
            .arg(Arg::with_name("readonly")
                 .long("readonly")
                 .help("Read-only instance of sit-web (no new items or records can be created)"))
            .arg(Arg::with_name("overlay")
                 .short("o")
                 .long("overlay")
                 .takes_value(true)
                 .multiple(true)
                 .help("Path to an additional [besides standard ones] web overlay"))
            .arg(Arg::with_name("listen")
                 .default_value("127.0.0.1:8080")
                 .help("Listen on IP:PORT"))));


    if allow_external_subcommands {
        app = app.setting(clap::AppSettings::AllowExternalSubcommands);
    }

    let matches = app.clone().get_matches();

    if let Some(matches) = matches.subcommand_matches("args") {
        return command_args::command(&matches);
   }

    let project_dirs = directories::ProjectDirs::from("fyi", "sit", "sit").expect("can't derive project directories");
    let default_config = project_dirs.config_dir().join("config.json");

    let config_path = matches.value_of("config").unwrap_or(default_config.to_str().unwrap());
    std::fs::create_dir_all(project_dirs.config_dir()).expect("can't ensure config directory's presence");

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name(config_path).required(false)).unwrap();

    let config: cfg::Configuration = settings.try_into().expect("can't load config");

    if matches.subcommand_name().is_none() {
        app.print_help().expect("can't print help");
        return 1;
    }

    let working_dir = PathBuf::from(matches.value_of("working_directory").unwrap_or(cwd.to_str().unwrap()));
    let dot_sit = working_dir.join(".sit");

    if let Some(matches) = matches.subcommand_matches("config") {
        if matches.value_of("kind").unwrap() == "user" {
            command_config::command(&config, matches.value_of("query"));
            return 0;
        }
    }

    if let Some(matches) = matches.subcommand_matches("jmespath") {
        return command_jmespath::command(matches);
    }

    if let Some(init_matches) = matches.subcommand_matches("init") {
        return command_init::command(&init_matches, &matches, &working_dir, &dot_sit);
    } else if let Some(matches) = matches.subcommand_matches("rebuild") {
        rebuild_repository(matches.value_of("SRC").unwrap(),
                           matches.value_of("DEST").unwrap(),
                           matches.value_of("on-record"));
        return 0;
    } else if let Some(_) = matches.subcommand_matches("upgrade") {
        let mut upgrades = vec![];
        let repo_path = matches.value_of("repository").map(PathBuf::from)
                       .or_else(|| sit_core::Repository::find_in_or_above(".sit",&working_dir))
                       .expect("Can't find a repository");
        loop {
            match sit_core::Repository::open_and_upgrade(&repo_path, &upgrades) {
                Err(sit_core::repository::Error::UpgradeRequired(upgrade)) => {
                    println!("{}", upgrade);
                    upgrades.push(upgrade);
                },
                Err(err) => {
                    eprintln!("Error occurred: {:?}", err);
                    return 1;
                },
                Ok(_) => break,
            }
        }
        return 0;
    } else {
        let repo_path = matches.value_of("repository").map(PathBuf::from)
                       .or_else(|| sit_core::Repository::find_in_or_above(".sit",&working_dir))
                       .expect("Can't find a repository");
        let mut repo = sit_core::Repository::open(&repo_path)
            .expect("can't open repository");
        let integrity_check = !matches.is_present("disable-integrity-check") && !env::var("SIT_DISABLE_INTEGRITY_CHECK").is_ok();
        repo.set_integrity_check(integrity_check);
        return match repo.config().clone().extra().get("external_module_manager") {
            Some(serde_json::Value::String(name)) => {
                let original_repo = repo.clone();
                do_matches(matches.clone(), repo.with_module_iterator(ScriptModule(original_repo, cwd.clone(), name.to_string())), cwd.clone(), config, config_path)
            }
            _ => do_matches(matches.clone(), repo, cwd.clone(), config, config_path),
        };

        fn do_matches<MI: 'static + Send + Sync>(matches: ArgMatches<'static>, repo: sit_core::Repository<MI>, cwd: PathBuf, config: cfg::Configuration, config_path: &str) -> i32
            where MI: sit_core::repository::ModuleIterator<PathBuf, sit_core::repository::Error> {
            let working_dir = PathBuf::from(matches.value_of("working_directory").unwrap_or(cwd.to_str().unwrap()));
            let canonical_working_dir = dunce::canonicalize(&working_dir).expect("can't canonicalize working directory");
            if let Some(_) = matches.subcommand_matches("modules") {
                match repo.module_iter() {
                    Ok(iter) => {
                        for module_path in iter {
                            let module_path = module_path.expect("can't get module_path");
                            println!("{}", ::dunce::canonicalize(&module_path).unwrap_or(module_path).to_str().unwrap());
                        }
                        return 0;
                    },
                    Err(sit_core::RepositoryError::OtherError(str)) => {
                        eprintln!("{}", str);
                        return 1;
                    },
                    Err(e) => {
                        eprintln!("Error: {:?}", e);
                        return 1;
                    },
                }
            }

            if let Some(_) = matches.subcommand_matches("populate-files") {
                repo.populate_default_files().expect("can't populate default files");
                return 0;
            } else if let Some(matches) = matches.subcommand_matches("path") {
                if let Some(id) = matches.value_of("record") {
                    match repo.record(id) {
                        None => {
                            eprintln!("Record {} not found", id);
                            return 1;
                        },
                        Some(record) => {
                            println!("{}", record.path().to_str().unwrap());
                            return 0;
                        }
                    }
                } else {
                    println!("{}", repo.path().to_str().unwrap());
                    return 0;
                }
            } else if let Some(matches) = matches.subcommand_matches("item") {
                return command_item::command(matches, &repo);
            }

            if let Some(matches) = matches.subcommand_matches("items") {
                return command_items::command(matches, &repo, config);
            }

            if let Some(matches) = matches.subcommand_matches("record") {
                return command_record::command(matches, &repo, config.clone(), canonical_working_dir, config_path);
            }

            if let Some(matches) = matches.subcommand_matches("records") {
                return command_records::command(matches, repo, config);
            }

            if let Some(matches) = matches.subcommand_matches("reduce") {
                return command_reduce::command(matches, repo, config);
            }

            if let Some(matches) = matches.subcommand_matches("config") {
                if matches.value_of("kind").unwrap() == "repository" {
                    command_config::command(repo.config(), matches.value_of("query"));
                }
                return 0;
            }

            if let Some(_) = matches.subcommand_matches("integrity") {
                return command_integrity::command(repo);
            }

            if let Some(web_matches) = matches.subcommand_matches("web") {
                return command_web::command(repo, web_matches, matches.clone(), config, canonical_working_dir, config_path);
            }

            match command_external::command(&matches, repo, &cwd) {
                Err(_) => {
                    return main_with_result(false)
                },
                Ok(code) => {
                    return code
                }
            }
        }
    }

}
