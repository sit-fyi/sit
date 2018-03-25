use jmespath;
use serde_json;
use serde::Serialize;

pub fn command<T: Serialize>(cfg: &T, query: Option<&str>) {
    match query {
        None => println!("{}", serde_json::to_string_pretty(cfg).unwrap()),
        Some(query_expr) => {
            let query = jmespath::compile(query_expr).expect("can't compile query expression");
            let view = query.search(&cfg).unwrap();
            if view.is_string() {
                println!("{}", view.as_string().unwrap().clone())
            } else {
                println!("{}", serde_json::to_string_pretty(&view).unwrap())
            }
        }
    }
}
