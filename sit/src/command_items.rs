use clap::ArgMatches;
use sit_core::{self, reducers::duktape::DuktapeReducer, Repository, record::RecordContainerReduction};
use cfg::Configuration;
use serde_json;
use rayon::prelude::*;
use super::get_named_expression;
use jmespath;

use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use thread_local::ThreadLocal;
use std::path::PathBuf;

pub fn command<MI: Send + Sync>(matches: &ArgMatches, repo: &Repository<MI>, config: Configuration) -> i32
    where MI: sit_core::repository::ModuleIterator<PathBuf, sit_core::repository::Error>
{
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

    let tl_reducer : ThreadLocal<RefCell<DuktapeReducer<sit_core::repository::Record>>> = ThreadLocal::new();
    let reducer = Arc::new(Mutex::new(DuktapeReducer::new(repo).unwrap()));

    items.into_par_iter()
        .map(|item| {
            let mut reducer = tl_reducer.get_or(|| Box::new(RefCell::new(reducer.lock().unwrap().clone()))).borrow_mut();
            reducer.reset_state();
            let result = item.reduce_with_reducer(&mut *reducer).expect("can't reduce item");
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
    0
}
