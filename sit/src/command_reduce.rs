use clap::ArgMatches;
use sit_core::{self, Repository, item::ItemReduction, cfg::Configuration};
use serde_json;
use super::get_named_expression;
use jmespath;

pub fn command(matches: &ArgMatches, repo: &Repository, config: Configuration) -> i32 {
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
