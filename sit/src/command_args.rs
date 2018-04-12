use clap::{App, ArgMatches, OsValues};
use yaml_rust;

pub fn command(matches: &ArgMatches) -> i32 {
    let mut buffer = String::new();
    use ::std::io::{self, Read};
    io::stdin().read_to_string(&mut buffer).expect("can't read stdin");
    let yaml = yaml_rust::YamlLoader::load_from_str(&buffer).expect("can't parse YAML");
    let mut app = App::from_yaml(&yaml[0]);
    if matches.is_present("help") {
        app.print_help().expect("can't print help");
    } else {
        let args = matches.values_of_os("ARGS").unwrap_or(OsValues::default());
        let matches = app.get_matches_from(args);
        let (subcommand, matches) = matches.subcommand();
        println!("{}", subcommand);
        if matches.is_some() {
            for (name, arg) in &matches.unwrap().args {
                print!("{} {} ", name, arg.occurs);
                for index in &arg.indices {
                    print!("{} ", index);
                }
                for val in &arg.vals {
                    print!("\"{}\" ", val.to_str().unwrap());
                }
                println!();
            }
        }
    }
    return 0;
}