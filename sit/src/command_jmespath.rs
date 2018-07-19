use clap::ArgMatches;
use jmespath;
use serde_json;

pub fn command(matches: &ArgMatches) -> i32 {
    let query = jmespath::compile(matches.value_of("expr").unwrap_or("@")).expect("can't compile expression");
    let data = jmespath::Variable::from(serde_json::from_reader::<_, serde_json::Value>(::std::io::stdin()).expect("can't parse JSON"));
    let result = query.search(&data).unwrap();
    if matches.is_present("pretty") {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        println!("{}", serde_json::to_string(&result).unwrap());
    }
    return 0;
}