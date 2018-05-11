use clap::ArgMatches;
use sit_core::{Repository, Record, Item, cfg::Configuration};
use serde_json;
use super::get_named_expression;
use jmespath;
use std::fs;
use super::gnupg;

pub fn command(matches: &ArgMatches, repo: &Repository, config: Configuration) -> i32 {
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
