extern crate sit_core;

extern crate chrono;
extern crate tempfile;
#[macro_use] extern crate clap;

use std::env;
use std::path::PathBuf;
use std::fs;
use std::process::exit;

use clap::{Arg, App, SubCommand, ArgMatches};

use sit_core::{Item, Record};
use sit_core::item::ItemReduction;

extern crate serde;
extern crate serde_json;
extern crate yaml_rust;

extern crate config;
use sit_core::cfg;

mod rebuild;
use rebuild::rebuild_repository;
mod command_config;
mod command_args;
mod command_init;
mod command_item;
mod command_record;
mod command_items;

#[cfg(unix)]
extern crate xdg;

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

use std::collections::HashMap;
pub fn get_named_expression<S: AsRef<str>>(name: S, repo: &sit_core::Repository,
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


fn main() {
    exit(main_with_result(true));
}

fn main_with_result(allow_external_subcommands: bool) -> i32 {
    #[cfg(unix)]
    let xdg_dir = xdg::BaseDirectories::with_prefix("sit").unwrap();

    let cwd = env::current_dir().expect("can't get current working directory");
    let mut app = App::new("SIT")
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
        .subcommand(SubCommand::with_name("upgrade")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Upgrades the repository"))
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
        .subcommand(SubCommand::with_name("item")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Creates a new item")
            .arg(Arg::with_name("id")
                     .long("id")
                     .takes_value(true)
                     .required(false)
                     .help("Specify item identifier, otherwise generate automatically")))
        .subcommand(SubCommand::with_name("items")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Lists items")
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
                     .help("Render a result of a named JMESPath query over the item")))
        .subcommand(SubCommand::with_name("record")
            .settings(&[clap::AppSettings::ColoredHelp, clap::AppSettings::ColorAuto])
            .about("Creates a new record")
            .arg(Arg::with_name("id")
                     .takes_value(true)
                     .required(true)
                     .help("Item identifier"))
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
                     .help("Item identifier"))
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
            .about("Reduce item records")
            .arg(Arg::with_name("id")
                     .takes_value(true)
                     .required(true)
                     .help("Item identifier"))
            .arg(Arg::with_name("query")
                     .conflicts_with("named-query")
                     .long("query")
                     .short("q")
                     .takes_value(true)
                     .help("Render a result of a JMESPath query over the item (defaults to `@`)"))
            .arg(Arg::with_name("named-query")
                     .conflicts_with("query")
                     .long("named-query")
                     .short("Q")
                     .takes_value(true)
                     .help("Render a result of a named JMESPath query over the item")))
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
            .about("Parses arguments against a specification given on stdin"));

    if allow_external_subcommands {
        app = app.setting(clap::AppSettings::AllowExternalSubcommands);
    }

    let matches = app.clone().get_matches();

    if let Some(matches) = matches.subcommand_matches("args") {
        return command_args::command(&matches);
   }

    #[cfg(unix)]
    let default_config = PathBuf::from(xdg_dir.place_config_file("config.json").expect("can't create config directory"));
    #[cfg(windows)]
    let default_config = env::home_dir().expect("can't identify home directory").join("sit_config.json");

    let config_path = matches.value_of("config").unwrap_or(default_config.to_str().unwrap());

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name(config_path).required(false)).unwrap();

    let config: cfg::Configuration = settings.try_into().expect("can't load config");

    if matches.subcommand_name().is_none() {
        app.print_help().expect("can't print help");
        return 1;
    }

    let working_dir = PathBuf::from(matches.value_of("working_directory").unwrap());
    let canonical_working_dir = dunce::canonicalize(&working_dir).expect("can't canonicalize working directory");
    let dot_sit = working_dir.join(".sit");

    if let Some(matches) = matches.subcommand_matches("config") {
        if matches.value_of("kind").unwrap() == "user" {
            command_config::command(&config, matches.value_of("query"));
            return 0;
        }
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
        let repo = sit_core::Repository::open(&repo_path)
            .expect("can't open repository");

        if let Some(_) = matches.subcommand_matches("modules") {
            for module_path in repo.module_iter().expect("can't iterate over modules") {
                println!("{}", ::dunce::canonicalize(&module_path).unwrap_or(module_path).to_str().unwrap());
            }
            return 0;
        }

        if let Some(_) = matches.subcommand_matches("populate-files") {
            repo.populate_default_files().expect("can't populate default files");
            return 0;
        } else if let Some(_) = matches.subcommand_matches("path") {
            println!("{}", repo.path().to_str().unwrap());
            return 0;
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
            let id = matches.value_of("id").unwrap();
            match repo.item(id) {
                None => {
                    eprintln!("Item {} not found", id);
                    return 1;
                },
                Some(item) => {
                    let records = item.record_iter().expect("can't lis records");

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
                                   use std::io::Write;
                                   let program = gnupg(matches, &config).expect("can't find GnuPG");
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
            return 0;
        }

        if let Some(matches) = matches.subcommand_matches("reduce") {
            let id = matches.value_of("id").unwrap();
            match repo.item(id) {
                None => {
                    eprintln!("Item {} not found", id);
                    return 1;
                },
                Some(item) => {
                    let query_expr = matches.value_of("named-query")
                        .and_then(|name|
                            get_named_expression(name, &repo, ".items/queries", &config.items.queries))
                        .or_else(|| matches.value_of("query").or_else(|| Some("@")).map(String::from))
                        .unwrap();

                    let query = jmespath::compile(&query_expr).expect("can't compile query expression");

                    let mut reducer = sit_core::reducers::duktape::DuktapeReducer::new(&repo).unwrap();
                    let result = item.reduce_with_reducer(&mut reducer).expect("can't reduce item");
                    let data = jmespath::Variable::from(serde_json::Value::Object(result));
                    let view = query.search(&data).unwrap();
                    if view.is_string() {
                        println!("{}", view.as_string().unwrap());
                    } else {
                        println!("{}", serde_json::to_string_pretty(&view).unwrap());
                    }

                }
            }
            return 0;
        }

        if let Some(matches) = matches.subcommand_matches("config") {
            if matches.value_of("kind").unwrap() == "repository" {
                command_config::command(repo.config(), matches.value_of("query"));
            }
            return 0;
        }

        let (subcommand, args) = matches.subcommand();

        #[cfg(not(windows))]
        let path_sep = ":";
        #[cfg(windows)]
        let path_sep = ";";

        let mut path: String = repo.path().join("cli").to_str().unwrap().into();
        for module_name in repo.module_iter().expect("can't iterate over modules") {
            path += path_sep;
            path += repo.modules_path().join(module_name).join("cli").to_str().unwrap().into();
        }

        path += path_sep;
        path += &env::var("PATH").unwrap();

        match which::which_in(format!("sit-{}", subcommand), Some(path), &cwd) {
            Ok(path) => {
                let mut command = ::std::process::Command::new(path);
                command.env("SIT_DIR", repo.path().to_str().unwrap());
                command.env("SIT", env::current_exe().unwrap_or("sit".into()).to_str().unwrap());
                if let Some(args) = args {
                    command.args(args.values_of_lossy("").unwrap_or(vec![]));
                }
                match command.spawn() {
                    Err(_) => {
                        return main_with_result(false);
                    },
                    Ok(mut process) => {
                        let result = process.wait().unwrap();
                        return result.code().unwrap();
                    },
                }
            },
            Err(_) => {
                return main_with_result(false);
            },
        }

    }

}
