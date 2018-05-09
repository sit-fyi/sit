use clap::ArgMatches;
use sit_core::{self, Repository, item::ItemReduction, cfg::Configuration};
use serde_json;
use rayon::prelude::*;
use super::get_named_expression;
use jmespath;

pub fn command(matches: &ArgMatches, repo: &Repository, config: Configuration) -> i32 {
    let items: Vec<_> = repo.item_iter().expect("can't list items").collect();

    let filter_expr = matches.value_of("named-filter")
        .and_then(|name|
            get_named_expression(name, &repo, ".items/filters", &config.items.filters))
        .or_else(|| matches.value_of("filter").or_else(|| Some("`true`")).map(String::from))
        .unwrap();

    let filter_defined = matches.is_present("named-filter") || matches.is_present("filter");

    let query_expr = matches.value_of("named-query")
        .and_then(|name|
            get_named_expression(name, &repo, ".items/queries", &config.items.queries))
        .or_else(|| matches.value_of("query").or_else(|| Some("id")).map(String::from))
        .unwrap();

    let filter = jmespath::compile(&filter_expr).expect("can't compile filter expression");
    let query = jmespath::compile(&query_expr).expect("can't compile query expression");

    let reducer = sit_core::reducers::duktape::DuktapeReducer::new(&repo).unwrap();
    let items_with_reducers: Vec<_> = items.into_iter().map(|i| (i, reducer.clone())).collect();
    items_with_reducers.into_par_iter()
        .map(|(item, mut reducer)| {
            let result = item.reduce_with_reducer(&mut reducer).expect("can't reduce item");
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
    return 0;
}
