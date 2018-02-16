#[allow(dead_code)]
mod assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));

    use rouille::{Response, ResponseBody};
    use mime_guess::get_mime_type_str;
    use std::path::PathBuf;

    impl<'a> Into<Response> for &'a File {
        fn into(self) -> Response {
            match get_mime_type_str(PathBuf::from(self.name()).extension().unwrap().to_str().unwrap()) {
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
            }
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

use rouille::{start_server, Response, ResponseBody};
use rouille::input::multipart::get_multipart_input;

use std::path::PathBuf;
use std::fs;
use std::net::ToSocketAddrs;

use sit_core::Repository;

use mime_guess::get_mime_type_str;

use std::ffi::OsString;

use rayon::prelude::*;

use tempdir;

fn path_to_response<P: Into<PathBuf>>(path: P) -> Response {
    let path: PathBuf = path.into();
    match get_mime_type_str(path.extension().unwrap_or(&OsString::new()).to_str().unwrap()) {
        None => Response {
            status_code: 200,
            headers: vec![("Content-Type".into(), "application/octet-stream".into())],
            data: ResponseBody::from_reader(fs::File::open(path).unwrap()),
            upgrade: None,
        },
        Some(content_type) => Response {
            status_code: 200,
            headers: vec![("Content-Type".into(), content_type.into())],
            data: ResponseBody::from_reader(fs::File::open(path).unwrap()),
            upgrade: None,
        },
    }
}

use itertools::Itertools;
use sit_core;

pub fn start<A: ToSocketAddrs>(addr: A, config: sit_core::cfg::Configuration, repo: Repository) {
    let assets: PathBuf = repo.path().join(".web").into();
    start_server(addr, move |request|
        router!(request,
        (GET) (/user/config) => {
          Response::json(&config)
        },
        (GET) (/api/issues/{filter_expr: String}/{query_expr: String}) => {
            use jmespath;
            use sit_core::issue::IssueReduction;
            let issues = repo.issue_iter().expect("can't list issues");
            let mut reducer = sit_core::reducers::duktape::DuktapeReducer::new(&repo).unwrap();
            let issues_with_reducers: Vec<_> =  issues.into_iter().map(|i| (i, reducer.clone())).collect();

            let filter = match jmespath::compile(&filter_expr) {
                Ok(filter) => filter,
                _ => return Response::empty_400(),
            };
            let query = match jmespath::compile(&query_expr) {
                Ok(query) => query,
                _ => return Response::empty_400(),
            };

            let result: Vec<_> =
            issues_with_reducers.into_par_iter()
                  .map(|(issue, mut reducer)| {
                     issue.reduce_with_reducer(&mut reducer).unwrap()
                  })
                  .map(|reduced| {
                     sit_core::serde_json::to_string(&reduced).unwrap()
                  }).map(|json| {
                     let data = jmespath::Variable::from_json(&json).unwrap();
                     let result = filter.search(&data).unwrap();
                     if result.is_boolean() && result.as_boolean().unwrap() {
                        Some(query.search(&data).unwrap())
                     } else {
                        None
                     }
                  })
                 .filter(Option::is_some).collect();
            Response::json(&result)
        },
        (GET) (/api/issue/{id: String}/{query_expr: String}) => {
            use jmespath;
            use sit_core::issue::IssueReduction;
            use sit_core::Issue;
            let mut reducer = sit_core::reducers::duktape::DuktapeReducer::new(&repo).unwrap();
            let query = match jmespath::compile(&query_expr) {
                Ok(query) => query,
                _ => return Response::empty_400(),
            };
            let issue = match repo.issue_iter().unwrap().find(|i| i.id() == id) {
                Some(issue) => issue,
                _ => return Response::empty_404(),
            };
            let reduced = issue.reduce_with_reducer(&mut reducer).unwrap();
            let json = sit_core::serde_json::to_string(&reduced).unwrap();
            let data = jmespath::Variable::from_json(&json).unwrap();
            let result = query.search(&data).unwrap();
            Response::json(&result)
        },
        (GET) (/api/issue/{id: String}/{record: String}/files) => {
            use sit_core::{Record, Issue};
            let issue = match repo.issue_iter().unwrap().find(|i| i.id() == id) {
                Some(issue) => issue,
                None => return Response::empty_404(),
            };
            let record = match issue.record_iter().unwrap().flatten().find(|r| r.encoded_hash() == record) {
               Some(record) => record,
               None => return Response::empty_404(),
            };
            let files: Vec<_> = record.file_iter().map(|(name, _)| name).collect();
            Response::json(&files)
        },
        (POST) (/api/issue) => {
           use sit_core::Issue;
           let issue = repo.new_issue().expect("can't create issue");
           Response::json(&issue.id())
        },
        (POST) (/api/issue/{id: String}/records) => {
           use sit_core::{Issue, Record};
           let mut issue = match repo.issue_iter().unwrap().find(|i| i.id() == id) {
                Some(issue) => issue,
                None => return Response::empty_404(),
           };

           let mut multipart = get_multipart_input(&request).expect("multipart request");
           let mut files = vec![];
           let mut link = true;
           let mut used_files = vec![];
           loop {
              let mut part = multipart.next();
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
                 files.push((field.name.clone(), fs::File::open(&path).expect("can't open saved file")));
                 used_files.push(path);
                 match field.next_entry_inplace() {
                     Ok(Some(_)) => continue,
                     Ok(None) => break,
                     Err(e) => panic!(e),
                 }
              }
           }

           let tmp = tempdir::TempDir::new_in(repo.path(), "sit").unwrap();
           let record_path = tmp.path();

           let record = issue.new_record_in(record_path, files.into_iter(), link).expect("can't create record");

           for file in used_files {
             fs::remove_file(file).expect("can't remove file");
           }

           if config.signing.enabled {
              use std::ffi::OsString;
              use std::io::Write;
              let program = match config.signing.gnupg {
                           Some(ref command) => command.clone(),
                           None => String::from("gpg"),
              };
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
                  stdin.write_all(record.encoded_hash().as_bytes()).expect("Failed to write to stdin");
              }

              let output = child.wait_with_output().expect("failed to read stdout");

              if !output.status.success() {
                  eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
              } else {
                  use sit_core::repository::DynamicallyHashable;
                  let dynamically_hashed_record = record.dynamically_hashed();
                  let mut file = fs::File::create(record.actual_path().join(".signature"))
                               .expect("can't open signature file");
                 file.write(&output.stdout).expect("can't write signature file");
                 drop(file);
                 let new_hash = dynamically_hashed_record.encoded_hash();
                 let mut new_path = record.path();
                 new_path.pop();
                 new_path.push(&new_hash);
                 fs::rename(record.actual_path(), new_path).expect("can't rename record");
                 return Response::json(&new_hash);
             }

          } else {
                 fs::rename(record.actual_path(), record.path()).expect("can't rename record");
          }

          Response::json(&record.encoded_hash())
        },
        _ => {
        // Serve repository content
        if request.url().starts_with("/repo/") {
            let file = repo.path().join(&request.url()[6..]);
            if file.strip_prefix(repo.path()).is_err() {
               return Response::empty_404();
            }
            if file.is_file() {
                return path_to_response(file)
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
        let overriden_path = assets.join(&request.url()[1..]);
        if overriden_path.is_file() {
           return path_to_response(overriden_path)
        } else {
            if let Some(file) = ASSETS.get(&PathBuf::from(&request.url()[1..])) {
                return file.into()
            }
        }
        // Route the rest to /index.html for the web app to figure out
        let custom_index = assets.join("index.html");
        if custom_index.is_file() {
           path_to_response(custom_index)
        } else {
           ASSETS.get(&PathBuf::from("index.html")).unwrap().into()
        }
      }
      ))

}

