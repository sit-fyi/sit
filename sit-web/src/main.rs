extern crate sit_core;
extern crate sit;

extern crate chrono;
extern crate dirs;
extern crate tempfile;
#[macro_use] extern crate clap;

use std::env;
use std::path::PathBuf;
use std::fs;
use std::process::exit;

use clap::{Arg, App};

extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

extern crate config;
use sit_core::cfg;

#[cfg(unix)]
extern crate xdg;

extern crate jmespath;

extern crate itertools;

extern crate rayon;

extern crate tempdir;

extern crate digest;
extern crate blake2;
extern crate hex;

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate rouille;
extern crate mime_guess;
mod webapp;

extern crate which;

use std::ffi::OsString;
use which::which;

use sit::ScriptModule;

extern crate thread_local;

pub fn gnupg(config: &cfg::Configuration) -> Result<OsString, which::Error> {
    let program = match config.signing.gnupg {
            Some(ref command) => command.clone().into(),
            None => which("gpg2").or_else(|_| which("gpg"))?.to_str().unwrap().into(),
    };
    Ok(program)
}

fn main() {
    #[cfg(unix)]
    let xdg_dir = xdg::BaseDirectories::with_prefix("sit").unwrap();

    let cwd = env::current_dir().expect("can't get current working directory");
    let matches = App::new("SIT Web Interface")
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
            .help("Listen on IP:PORT"))
        .get_matches();


    #[cfg(unix)]
    let default_config = PathBuf::from(xdg_dir.place_config_file("config.json").expect("can't create config directory"));
    #[cfg(windows)]
    let default_config = dirs::home_dir().expect("can't identify home directory").join("sit_config.json");

    let config_path = matches.value_of("config").unwrap_or(default_config.to_str().unwrap());

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name(config_path).required(false)).unwrap();

    let mut config: cfg::Configuration = settings.try_into().expect("can't load config");

    let working_dir = PathBuf::from(matches.value_of("working_directory").unwrap());
    let canonical_working_dir = fs::canonicalize(&working_dir).expect("can't canonicalize working directory");

    if config.author.is_none() {
        if let Some(author) = cfg::Author::from_gitconfig(canonical_working_dir.join(".git/config")) {
            config.author = Some(author);
        } else if let Some(author) = cfg::Author::from_gitconfig(dirs::home_dir().expect("can't identify home directory").join(".gitconfig")) {
            config.author = Some(author);
        } else {
            eprintln!("Authorship hasn't been configured. Update your {} config file\n\
            to include `author` object with `name` and `email` properties specified", config_path);
            exit(1);
        }
    }

    let repo_path = matches.value_of("repository").map(PathBuf::from)
        .or_else(|| sit_core::Repository::find_in_or_above(".sit",&working_dir))
        .expect("Can't find a repository");
    let repo = sit_core::Repository::open(&repo_path)
        .expect("can't open repository");

    let listen = matches.value_of("listen").unwrap();
    let readonly = matches.is_present("readonly");
    let overlays: Vec<_> = matches.values_of("overlay").unwrap_or(clap::Values::default()).collect();
    println!("Serving on {}", listen);
    match repo.config().clone().extra().get("module_manager") {
            Some(serde_json::Value::String(name)) => {
                let original_repo = repo.clone();
                let repo = repo.with_module_iterator(ScriptModule(original_repo, cwd.clone(), name.clone()));
                webapp::start(listen, config, repo, readonly, overlays);
            },
            _ => webapp::start(listen, config, repo, readonly, overlays),
    };

}
