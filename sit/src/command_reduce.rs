use clap::{ArgMatches, Values};
use sit_core::{self, Repository, record::RecordContainerReduction, repository, 
               reducers::duktape, path::{HasPath, ResolvePath}};
use crate::cfg::Configuration;
use serde_json;
use super::get_named_expression;
use jmespath;
use std::path::PathBuf;

pub fn command<MI>(matches: &ArgMatches, repo: Repository<MI>, config: Configuration) -> i32
    where MI: repository::ModuleIterator<PathBuf, repository::Error> {
    if let Some(vals) = matches.values_of_os("reducer") {
        let reducers_path = repo.path().join("reducers");
        let reducers = vals.map(PathBuf::from)
            .map(|p| if p.is_file() {
                p
            } else if reducers_path.join(&p).resolve_dir("/").unwrap().is_dir() {
                let dir = reducers_path.join(&p).resolve_dir("/").unwrap();
                dir
            } else {
                p
            });
        command_impl(matches, &repo, config, reducers)
    } else {
        command_impl(matches, &repo, config, &repo)
    }
}

fn command_impl<MI, SF>(matches: &ArgMatches, repo: &Repository<MI>, config: Configuration, source_files: SF) -> i32
    where MI: repository::ModuleIterator<PathBuf, repository::Error>, SF: duktape::SourceFiles {

    let fixed_roots = matches.values_of("root");
    let state = matches.value_of("state").map(serde_json::from_str).filter(Result::is_ok).map(Result::unwrap);

    #[cfg(feature = "deprecated-items")] {
        if let Some(id) = matches.value_of("id") {
            match repo.item(id) {
                None => {
                    eprintln!("Item {} not found", id);
                    return 1;
                },
                Some(item) => {
                    let query_expr = matches.value_of("named-query")
                        .and_then(|name|
                            get_named_expression(name, repo, ".items/queries", &config.items.queries))
                        .or_else(|| matches.value_of("query").or_else(|| Some("@")).map(String::from))
                        .unwrap();

                    reduce(&query_expr, &item, source_files, fixed_roots, state);
                    return 0;
                }
            }
        }
    }

    let query_expr = matches.value_of("named-query")
        .and_then(|name|
            get_named_expression(name, repo, ".queries", &config.items.queries))
        .or_else(|| matches.value_of("query").or_else(|| Some("@")).map(String::from))
        .unwrap();

    reduce(&query_expr, repo, source_files, fixed_roots, state);

    return 0;
}

fn reduce<RCR: RecordContainerReduction<Record = repository::Record>, SF: duktape::SourceFiles>
    (query_expr: &str, container: &RCR, source_files: SF, roots: Option<Values>, state: Option<serde_json::Value>) {
    let mut reducer = sit_core::reducers::duktape::DuktapeReducer::new(source_files).unwrap();
    let query = jmespath::compile(&query_expr).expect("can't compile query expression");
    let state = container.initialize_state(match state {
        None => Default::default(),
        Some(s) => s.as_object().unwrap().to_owned(),
    });
    let result = match roots {
        None => container.reduce_with_reducer_and_state(&mut reducer, state).expect("can't reduce"),
        Some(fixed_roots) => {
            let container = container.fixed_roots(fixed_roots);
            container.reduce_with_reducer_and_state(&mut reducer, state).expect("can't reduce")
        },
    };
    let data = jmespath::Variable::from(serde_json::Value::Object(result));
    let view = query.search(&data).unwrap();
    if view.is_string() {
        println!("{}", view.as_string().unwrap());
    } else {
        println!("{}", serde_json::to_string_pretty(&view).unwrap());
    }
}
