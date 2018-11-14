use clap::{self, ArgMatches};
use sit_core::{Repository, repository};
use crate::cfg::Configuration;
use crate::authorship::derive_authorship;
use std::path::{Path, PathBuf};

pub fn command<MI: 'static + Send + Sync, P: AsRef<Path>>(repo: Repository<MI>, matches: &ArgMatches, main_matches: ArgMatches<'static>, mut config: Configuration, config_path: P) -> i32 
    where MI: repository::ModuleIterator<PathBuf, repository::Error> {
    {
        let result = derive_authorship(&mut config, config_path.as_ref());
        if result != 0 {
            return result;
        }
    }
    let listen = matches.value_of("listen").unwrap();
    let readonly = matches.is_present("readonly");
    let overlays: Vec<_> = matches.values_of("overlay").unwrap_or(clap::Values::default()).collect();
    println!("Serving on {}", listen);
    webapp::start(listen, config, repo, readonly, overlays, main_matches);
    return 0;
}

mod webapp {
    use crate::cfg;
    #[allow(dead_code)]
    mod assets {
        include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));

        use rouille::{Response, ResponseBody};
        use mime_guess::get_mime_type_str;
        use std::path::PathBuf;
        use blake2::Blake2b;
        use digest::{Input, VariableOutput};
        use hex;

        impl<'a> Into<(Response, String)> for &'a File {
            fn into(self) -> (Response, String) {
                let mut hasher = Blake2b::new(20).unwrap();
                let mut result = vec![0; 20];
                hasher.process(self.contents);
                let hash = hex::encode(hasher.variable_result(&mut result).unwrap());
                (match get_mime_type_str(PathBuf::from(self.name()).extension().unwrap().to_str().unwrap()) {
                    None => Response {
                        status_code: 200,
                        headers: vec![("Content-Type".into(), "application/octet-stream".into())],
                        data: ResponseBody::from_data(self.contents),
                        upgrade: None,
                    },
                    Some(content_type) => Response {
                        status_code: 200,
                        headers: vec![("Content-Type".into(), content_type.into())],
                        data: ResponseBody::from_data(self.contents),
                        upgrade: None,
                    },
                }, hash)
            }
        }

        use std::collections::HashMap;

        lazy_static! {
            pub static ref ASSETS: HashMap<PathBuf, File> = {
                let mut map = HashMap::new();
                let mut prefix = PathBuf::from(FILES.find("index.html").unwrap().path());
                prefix.pop();
                for entry in FILES.walk() {
                    match entry {
                        DirEntry::File(f) => {
                            let path = PathBuf::from(f.path().strip_prefix(&prefix).unwrap());
                            map.insert(path.clone(), f.clone());
                            let super_path = PathBuf::from("super").join(path);
                            map.insert(super_path, f.clone());
                        },
                        _ => (),
                    }
                }
                map
            };
        }

    }
    use self::assets::ASSETS;

    use rouille::{start_server, Request, Response, ResponseBody};
    use rouille::input::multipart::get_multipart_input;

    use std::path::PathBuf;
    use std::fs;
    use std::net::ToSocketAddrs;

    use sit_core::{Repository, repository, reducers::duktape::{self, DuktapeReducer}, record::OrderedFiles,
    record::{RecordContainer, RecordContainerReduction, RecordOwningContainer}, path::{HasPath, ResolvePath}};
    use std::io::Cursor;

    use mime_guess::get_mime_type_str;

    use std::ffi::OsString;

    use rayon::prelude::*;

    use blake2::Blake2b;
    use digest::{Input, VariableOutput};
    use hex;

    use serde_json;

    use std::sync::{Arc, Mutex};
    use std::cell::RefCell;
    use thread_local::ThreadLocal;
    use clap::ArgMatches;

    fn path_to_response<P: Into<PathBuf>>(path: P, request: &Request) -> Response {
        let path: PathBuf = path.into();

        let mut file = fs::File::open(&path).unwrap();
        let mut buf = Vec::with_capacity(file.metadata().unwrap().len() as usize);
        use std::io::Read;
        file.read_to_end(&mut buf).unwrap();


        let mut hasher = Blake2b::new(20).unwrap();
        let mut result = vec![0; 20];
        hasher.process(&buf);
        let hash = hex::encode(hasher.variable_result(&mut result).unwrap());

        match get_mime_type_str(path.extension().unwrap_or(&OsString::new()).to_str().unwrap()) {
            None => Response {
                status_code: 200,
                headers: vec![("Content-Type".into(), "application/octet-stream".into())],
                data: ResponseBody::from_data(buf),
                upgrade: None,
            },
            Some(content_type) => Response {
                status_code: 200,
                headers: vec![("Content-Type".into(), content_type.into())],
                data: ResponseBody::from_data(buf),
                upgrade: None,
            },
        }.with_etag(request, hash)
    }


    use itertools::Itertools;
    use sit_core;

#[derive(Serialize)]
    struct Config {
        readonly: bool,
    }


    fn new_record<C: RecordOwningContainer, MI>(container: &C, request: &Request, repo: &Repository<MI>, config: &cfg::Configuration, matches: &ArgMatches) -> Result<C::Record, String> {
        let mut multipart = get_multipart_input(request).expect("multipart request");
        let mut link = true;
        let mut used_files = vec![];

        loop {
            let part = multipart.next();
            if part.is_none() {
                break;
            }
            let mut field = part.unwrap();
            loop {
                let path = {
                    let mut file = field.data.as_file().expect("files only");
                    let saved_file = file.save().temp().into_result().expect("can't save file");
                    saved_file.path
                };
                if field.name.starts_with(".prev/") {
                    link = false;
                }
                used_files.push((field.name.clone(), path));
                match field.next_entry_inplace() {
                    Ok(Some(_)) => continue,
                    Ok(None) => break,
                    Err(e) => panic!(e),
                }
            }
        }

        let files: OrderedFiles<_> = used_files.iter().map(|(n, p)| (n.clone(), fs::File::open(p).expect("can't open saved file"))).into();
        let files_: OrderedFiles<_> = used_files.iter().map(|(n, p)| (n.clone(), fs::File::open(p).expect("can't open saved file"))).into();

        let files: OrderedFiles<_> = if config.signing.enabled {
            use std::ffi::OsString;
            use std::io::Write;
            let program = super::super::gnupg(matches, &config).unwrap();
            let key = match config.signing.key.clone() {
                Some(key) => Some(OsString::from(key)),
                None => None,
            };

            let mut command = ::std::process::Command::new(program);

            command
                .stdin(::std::process::Stdio::piped())
                .stdout(::std::process::Stdio::piped())
                .arg("--sign")
                .arg("--armor")
                .arg("--detach-sign")
                .arg("-o")
                .arg("-");

            if key.is_some() {
                let _ = command.arg("--default-key").arg(key.unwrap());
            }

            let mut child = command.spawn().expect("failed spawning gnupg");

            {
                let mut stdin = child.stdin.as_mut().expect("Failed to open stdin");
                let mut hasher = repo.config().hashing_algorithm().hasher();
                files_.hash(&mut *hasher).expect("failed hashing files");
                let hash = hasher.result_box();
                let encoded_hash = repo.config().encoding().encode(&hash);
                stdin.write_all(encoded_hash.as_bytes()).expect("Failed to write to stdin");
            }

            let output = child.wait_with_output().expect("failed to read stdout");

            if !output.status.success() {
                eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
                return Err(String::from_utf8_lossy(&output.stderr).into());
            } else {
                let sig: OrderedFiles<_> = vec![(String::from(".signature"), Cursor::new(output.stdout))].into();
                files + sig
            }

        } else {
            files.boxed()
        };

        let record = container.new_record(files, link).expect("can't create record");

        for (_, file) in used_files {
            fs::remove_file(file).expect("can't remove file");
        }

        Ok(record)
    }

    fn reduce<MI, RCR: RecordContainerReduction<Record = repository::Record>>
        (repo: &Repository<MI>, container: &RCR, request: &Request, query_expr: String) -> Response
            where MI: repository::ModuleIterator<PathBuf, repository::Error> {
                if let Some(vals) = request.get_param("reducers") {
                    let reducers_path = repo.path().join("reducers");
                    let reducers = vals.split(",").map(PathBuf::from)
                        .map(|p| if p.is_file() {
                            p
                        } else if reducers_path.join(&p).resolve_dir("/").unwrap().is_dir() {
                            let dir = reducers_path.join(&p).resolve_dir("/").unwrap();
                            dir
                        } else {
                            p
                        });
                    return reduce__(container, request, query_expr, reducers)
                } else {
                    return reduce__(container, request, query_expr, repo)
                }
                // implementation
                fn reduce__<RCR: RecordContainerReduction<Record = repository::Record>, SF: duktape::SourceFiles>
                    (container: &RCR, request: &Request, query_expr: String, source_files: SF) -> Response {
                        use jmespath;
                        let reducer = sit_core::reducers::duktape::DuktapeReducer::new(source_files).unwrap();
                        let query = match jmespath::compile(&query_expr) {
                            Ok(query) => query,
                            _ => return Response::empty_400(),
                        };
                        fn reduce_<RCR: RecordContainerReduction<Record = repository::Record>>
                            (container: &RCR, query: jmespath::Expression, mut reducer: duktape::DuktapeReducer<repository::Record>, state: serde_json::Value)-> Response {
                                let state = container.initialize_state(state.as_object().unwrap().to_owned());
                                let reduced = container.reduce_with_reducer_and_state(&mut reducer, state).unwrap();
                                let data = jmespath::Variable::from(serde_json::Value::Object(reduced));
                                let result = query.search(&data).unwrap();
                                Response::json(&result)
                            }
                        if let Some(state) = request.get_param("state") {
                            reduce_(container, query, reducer, serde_json::from_str(&state).unwrap())
                        } else {
                            reduce_(container, query, reducer, serde_json::Value::Object(Default::default()))
                        }
                    }
            }


    pub fn start<A: ToSocketAddrs, MI: 'static + Send + Sync>(addr: A, config: cfg::Configuration, repo: Repository<MI>, readonly: bool, overlays: Vec<&str>, matches: ArgMatches<'static>)
        where MI: sit_core::repository::ModuleIterator<PathBuf, sit_core::repository::Error> {
            let mut overlays: Vec<_> = overlays.iter().map(|o| PathBuf::from(o)).collect();
            let assets: PathBuf = repo.path().join("web").into();
            overlays.push(assets);
            match repo.module_iter() {
                Ok(iter) => {
                    for module_name in iter {
                        let module_name = module_name.unwrap();
                        overlays.push(repo.modules_path().join(module_name).join("web").into());
                    }
                },
                Err(sit_core::RepositoryError::OtherError(str)) => {
                    eprintln!("{}", str);
                    return;
                },
                Err(e) => {
                    eprintln!("error: {:?}", e);
                    return;
                }
            }
            let repo_config = Config {
                readonly,
            };
            start_server(addr, move |request|
                         router!(request,
                                 (GET) (/user/config) => {
                                     Response::json(&config)
                                 },
                                 (GET) (/config) => {
                                     Response::json(&repo_config)
                                 },
                                 (GET) (/api/items/{filter_expr: String}/{query_expr: String}) => { // DEPRECATED
                                     #[cfg(feature = "deprecated-items")] {
                                         use jmespath;
                                         use sit_core::record::RecordContainerReduction;
                                         let items: Vec<_> = repo.item_iter().expect("can't list items").collect();
                                         let mut reducer = Arc::new(Mutex::new(sit_core::reducers::duktape::DuktapeReducer::new(&repo).unwrap()));
                                         let tl_reducer: ThreadLocal<RefCell<DuktapeReducer<sit_core::repository::Record>>>= ThreadLocal::new();

                                         let filter_defined = filter_expr != "";
                                         let filter = if filter_defined {
                                             match jmespath::compile(&filter_expr) {
                                                 Ok(filter) => filter,
                                                 _ => return Response::empty_400(),
                                             }
                                         } else {
                                             jmespath::compile("`true`").unwrap()
                                         };
                                         let query = match jmespath::compile(&query_expr) {
                                             Ok(query) => query,
                                             _ => return Response::empty_400(),
                                         };

                                         let result: Vec<_> =
                                             items.into_par_iter()
                                             .map(|item| {
                                                 let mut reducer = tl_reducer.get_or(|| Box::new(RefCell::new(reducer.lock().unwrap().clone()))).borrow_mut();
                                                 reducer.reset_state();
                                                 item.reduce_with_reducer(&mut *reducer).unwrap()
                                             }).map(|json| {
                                                 let data = jmespath::Variable::from(serde_json::Value::Object(json));
                                                 let result = if filter_defined {
                                                     let res = filter.search(&data).unwrap();
                                                     res.is_boolean() && res.as_boolean().unwrap()
                                                 } else {
                                                     true
                                                 };
                                                 if result {
                                                     Some(query.search(&data).unwrap())
                                                 } else {
                                                     None
                                                 }
                                             })
                                         .filter(Option::is_some).collect();
                                         Response::json(&result)
                                     }
                                     #[cfg(not(feature = "deprecated-items"))] {
                                         Response::not_found()
                                     }
                                 },
                                 (GET) (/api/item/{id: String}/{query_expr: String}) => { // DEPRECATED 
                                     #[cfg(feature = "deprecated-items")] {
                                         use jmespath;
                                         use sit_core::record::RecordContainerReduction;
                                         use sit_core::Item;
                                         let mut reducer = sit_core::reducers::duktape::DuktapeReducer::new(&repo).unwrap();
                                         let query = match jmespath::compile(&query_expr) {
                                             Ok(query) => query,
                                             _ => return Response::empty_400(),
                                         };
                                         let item = match repo.item_iter().unwrap().find(|i| i.id() == id) {
                                             Some(item) => item,
                                             _ => return Response::empty_404(),
                                         };
                                         let reduced = item.reduce_with_reducer(&mut reducer).unwrap();
                                         let data = jmespath::Variable::from(serde_json::Value::Object(reduced));
                                         let result = query.search(&data).unwrap();
                                         Response::json(&result)
                                     }
                                     #[cfg(not(feature = "deprecated-items"))] {
                                         Response::not_found()
                                     }
                                 },
                                 (GET) (/api/{roots: String}/reduce/{query_expr: String}) => {
                                     let container = repo.fixed_roots(roots.split(","));
                                     reduce(&repo, &container, &request, query_expr)
                                 },
                                 (GET) (/api/reduce/{query_expr: String}) => {
                                     reduce(&repo, &repo, &request, query_expr)
                                 },
                                 (GET) (/api/item/{id: String}/{record: String}/files) => { // DEPRECATED
                                     #[cfg(feature = "deprecated-items")] {
                                         use sit_core::{Record, Item};
                                         let item = match repo.item_iter().unwrap().find(|i| i.id() == id) {
                                             Some(item) => item,
                                             None => return Response::empty_404(),
                                         };
                                         let record = match ::itertools::Itertools::flatten(item.record_iter().unwrap()).find(|r| r.encoded_hash() == record) {
                                             Some(record) => record,
                                             None => return Response::empty_404(),
                                         };
                                         let files: Vec<_> = record.file_iter().map(|(name, _)| name).collect();
                                         Response::json(&files)
                                     }
                                     #[cfg(not(feature = "deprecated-items"))] {
                                         Response::not_found()
                                     }
                                 },
                                 (GET) (/api/record/{record: String}/files) => {
                                     use sit_core::Record;
                                     let record = match repo.record(record) {
                                         Some(record) => record,
                                         None => return Response::empty_404(),
                                     };
                                     let files: Vec<_> = record.file_iter().map(|(name, _)| name).collect();
                                     Response::json(&files)
                                 },
                                 (POST) (/api/item) => {
                                     #[cfg(feature = "deprecated-items")] { // DEPRECATED
                                         if readonly { return Response::empty_404(); }
                                         use sit_core::Item;
                                         let item = repo.new_item().expect("can't create item");
                                         Response::json(&item.id())
                                     }
                                     #[cfg(not(feature = "deprecated-items"))] {
                                         Response::not_found()
                                     }
                                 },
                                 (POST) (/api/item/{id: String}/records) => { // DEPRECATED
                                     #[cfg(feature = "deprecated-items")] {
                                         if readonly { return Response::empty_404(); }
                                         use sit_core::{Item, Record};
                                         let mut item = match repo.item_iter().unwrap().find(|i| i.id() == id) {
                                             Some(item) => item,
                                             None => return Response::empty_404(),
                                         };

                                         match new_record(&item, &request, &repo, &config, &matches) {
                                             Ok(record) => Response::json(&record.encoded_hash()),
                                             Err(_) => Response::text("Error").with_status_code(500),
                                         }
                                     }
                                     #[cfg(not(feature = "deprecated-items"))] {
                                         Response::not_found()
                                     }
                                 },
                                 (POST) (/api/records) => {
                                     if readonly { return Response::empty_404(); }
                                     use sit_core::Record;

                                     match new_record(&repo, &request, &repo, &config, &matches) {
                                         Ok(record) => Response::json(&record.encoded_hash()),
                                         Err(_) => Response::text("Error").with_status_code(500),
                                     }
                                 },
                                 _ => {
                                     // Serve repository content
                                     if request.url().starts_with("/repo/") {
                                         let mut file = repo.path().join(&request.url()[6..]);
                                         file = file.resolve_dir(repo.path()).unwrap_or(file);
                                         if file.strip_prefix(repo.path()).is_err() {
                                             return Response::empty_404();
                                         }
                                         if file.is_file() {
                                             return path_to_response(file, request)
                                         } else if file.is_dir() {
                                             if let Ok(dir) = ::std::fs::read_dir(file) {
                                                 let res = dir.filter(Result::is_ok)
                                                     .map(Result::unwrap)
                                                     .map(|e| if e.file_type().unwrap().is_dir() {
                                                         let s = String::from(e.file_name().to_str().unwrap());
                                                         (s + "/").into()
                                                     } else {
                                                         e.file_name()
                                                     })
                                                 .map(|s|
                                                      String::from(s.to_str().unwrap())
                                                     )
                                                     .join("\n");
                                                 return Response {
                                                     status_code: 200,
                                                     headers: vec![],
                                                     data: ResponseBody::from_string(res),
                                                     upgrade: None,
                                                 }
                                             }
                                         }
                                         return Response::empty_404()
                                     }
                                     // Serve built-in or overridden assets
                                     let overriden_path =
                                         overlays.iter().map(|o| o.join(&request.url()[1..]))
                                         .find(|p| p.is_file());
                                     if let Some(path) = overriden_path {
                                         return path_to_response(path, request)
                                     } else {
                                         if let Some(file) = ASSETS.get(&PathBuf::from(&request.url()[1..])) {
                                             let (response, hash) = file.into();
                                             return response.with_etag(request, hash)
                                         }
                                     }
                                     // Route the rest to /index.html for the web app to figure out
                                     let custom_index =
                                         overlays.iter().map(|o| o.join("index.html"))
                                         .find(|p| p.is_file());

                                     if let Some(index) = custom_index {
                                         path_to_response(index, request)
                                     } else {
                                         let (response, hash) = ASSETS.get(&PathBuf::from("index.html")).unwrap().into();
                                         response.with_etag(request, hash)
                                     }
                                 }
            ))

        }

}
