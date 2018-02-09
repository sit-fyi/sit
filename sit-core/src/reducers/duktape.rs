use std::io::Read;

use super::Reducer;
use serde_json::{Map, Value as JsonValue};
use std::marker::PhantomData;
use ::Record;
use duktape;
use std::ptr;
use std::ffi::CString;
use std::path::PathBuf;

#[derive(Debug)]
pub struct DuktapeReducer<'a, R: Record> {
    #[allow(dead_code)]
    repository: &'a ::Repository,
    context: *mut duktape::duk_context,
    reducers: usize,
    filenames: Vec<PathBuf>,
    phantom_data: PhantomData<R>,
}

#[derive(Debug, Error)]
pub enum Error {
    IoError(::std::io::Error),
    #[error(no_from, non_std)]
    CompileError {
        file: PathBuf,
        error: String,
    },
}

impl<'a, R: Record> Drop for DuktapeReducer<'a, R> {
    fn drop(&mut self) {
        unsafe {
            duktape::duk_destroy_heap(self.context);
        }
    }
}
unsafe extern "C" fn fatal_handler(_udata: *mut ::std::os::raw::c_void, msg: *const ::std::os::raw::c_char) {
    eprintln!("duktape aborted: {}", ::std::ffi::CStr::from_ptr(msg).to_str().unwrap());
    ::std::process::exit(1);
}

impl<'a, R: Record> DuktapeReducer<'a, R> {
    pub fn new(repository: &'a ::Repository) -> Result<Self, Error> {
        let context = unsafe {
            duktape::duk_create_heap(None, None, None,ptr::null_mut(), Some(fatal_handler))
        };
        use glob;
        use std::fs;
        let paths = glob::glob(repository.path().join(".reducers/*.js").to_str().unwrap()).unwrap();
        let mut reducers = 0;
        let mut filenames = vec![];
        for file in paths.filter(Result::is_ok).map(Result::unwrap) {
            reducers += 1;
            unsafe {
                // source code
                let mut source = String::new();
                let mut f = fs::File::open(&file)?;
                let _ = f.read_to_string(&mut source)?;
                let source = CString::new(source).unwrap();
                duktape::duk_push_string(context, source.as_ptr());

                // file name
                filenames.push(file.clone());
                let src_file = CString::new(String::from(file.to_str().unwrap())).unwrap();
                duktape::duk_push_string(context, src_file.as_ptr());

                // compile
                let res = duktape::duk_compile_raw(context, ptr::null_mut(), 0,
                                         duktape::DUK_COMPILE_SAFE |
                                         duktape::DUK_COMPILE_FUNCTION | duktape::DUK_COMPILE_STRLEN);

                if res as u32 == duktape::DUK_EXEC_ERROR {
                    let err = ::std::ffi::CStr::from_ptr(duktape::duk_safe_to_lstring(context, -1, ptr::null_mut())).to_str().unwrap();
                    return Err(Error::CompileError { file, error: err.into() })
                } else {
                    // clean up safe compilation results
                    // . . f
                    duktape::duk_swap_top(context, -2);
                    // . f .
                    duktape::duk_swap(context, -3, -2);
                    // f . .
                    duktape::duk_pop_2(context);
                    // f
                    duktape::duk_require_function(context, -1);
                }

                // create reducer's state
                duktape::duk_push_object(context);
                duktape::duk_require_function(context, -2);
                duktape::duk_require_object(context, -1);
            }
        }
        Ok(DuktapeReducer {
            repository,
            context,
            reducers,
            filenames,
            phantom_data: PhantomData,
        })
    }
}

impl<'a, R: Record> Reducer for DuktapeReducer<'a, R> {
    type State = Map<String, JsonValue>;
    type Item = R;

    fn reduce(&self, mut state: Self::State, item: &Self::Item) -> Self::State {
        use serde_json;

        let json = serde_json::to_string(&JsonValue::Object(state.clone())).unwrap();
        unsafe {
            let ctx = self.context;
            let cstring = CString::new(json).unwrap();

            // Item (record) TODO: complete
            duktape::duk_push_object(ctx);
            // item.hash
            let hash = CString::new(item.encoded_hash().as_ref()).unwrap();
            duktape::duk_push_string(ctx, hash.as_ptr());
            let hash_prop = CString::new("hash").unwrap();
            duktape::duk_put_prop_string(ctx, -2, hash_prop.as_ptr());
            // item.files
            duktape::duk_push_object(ctx);
            for (name, mut reader) in item.file_iter() {
                let filename = CString::new(name.as_ref()).unwrap();
                use std::io::Read;
                // INEFFICIENT BUT WORKS FOR NOW {
                let mut buf = vec![];
                let sz = reader.read_to_end(&mut buf).unwrap();
                let ptr = duktape::duk_push_buffer_raw(ctx,sz, 0);
                ptr::copy_nonoverlapping(buf.as_ptr(), ptr.offset(0) as *mut _, sz);
                // }
                duktape::duk_put_prop_string(ctx, -2, filename.as_ptr());
            }
            let files_prop = CString::new("files").unwrap();
            duktape::duk_put_prop_string(ctx, -2, files_prop.as_ptr());


            // Current issue state
            duktape::duk_push_string(ctx, cstring.as_ptr());
            duktape::duk_json_decode(ctx, -1);

            for i in 0..self.reducers {
                // function
                duktape::duk_require_function(ctx, (i * 2) as i32);
                duktape::duk_dup(ctx, (i * 2) as i32);
                // reducer's state
                duktape::duk_require_object(ctx,(i * 2 + 1) as i32);
                duktape::duk_dup(ctx, (i * 2 + 1) as i32);
                // issue state
                duktape::duk_push_null(ctx);
                duktape::duk_swap_top(ctx, -4);
                duktape::duk_require_object(ctx, -1);
                // item (record)
                duktape::duk_dup(ctx, -5);
                duktape::duk_require_object(ctx, -1);

                // execute
                let res = duktape::duk_pcall_method(ctx,2);


                // drop null state
                duktape::duk_swap_top(ctx, -2);
                duktape::duk_pop(ctx);

                // now it should be [item, current issue state] again

                // now, check for error
                if res as u32 == duktape::DUK_EXEC_ERROR {
                    let err = ::std::ffi::CStr::from_ptr(duktape::duk_safe_to_lstring(ctx, -1, ptr::null_mut()));
                    {
                        let mut arr = state.entry(String::from("errors")).or_insert(JsonValue::Array(vec![]));
                        let mut error = Map::new();
                        error.insert("file".into(), JsonValue::String(self.filenames[i].to_str().unwrap().into()));
                        error.insert("error".into(), JsonValue::String(err.to_str().unwrap().into()));
                        arr.as_array_mut().unwrap().push(JsonValue::Object(error));
                    }
                    return state;
                }

            }

            // remove item
            duktape::duk_swap_top(ctx,-2);
            duktape::duk_pop(ctx);

            // jsonify state
            duktape::duk_json_encode(ctx, -1);
            let json = duktape::duk_get_string(ctx, -1);

            let json = ::std::ffi::CStr::from_ptr(json);
            let map: Map<String, JsonValue> = serde_json::from_slice(json.to_bytes()).unwrap();

            // drop the json
            duktape::duk_pop(ctx);

            map
        }
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use super::*;
    use ::Repository;
    use issue::{Issue, IssueReduction};

    #[test]
    fn record_hash() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join(".reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join(".reducers/reducer.js")).unwrap();
        f.write(b"function(state, record) { return {\"hello\": record.hash}; }").unwrap();

        let issue = repo.new_issue().unwrap();
        let record = issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(DuktapeReducer::new(&repo).unwrap()).unwrap();

        assert_eq!(state.get("hello").unwrap(), &JsonValue::String(record.encoded_hash()));
    }


    #[test]
    fn record_contents() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join(".reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join(".reducers/reducer.js")).unwrap();
        f.write(b"function(state, record) { return {\"hello\": new TextDecoder('utf-8').decode(record.files.text)}; }").unwrap();

        let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(DuktapeReducer::new(&repo).unwrap()).unwrap();

        assert_eq!(state.get("hello").unwrap(), &JsonValue::String("Title".into()));
    }


    #[test]
    fn reducer_state() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join(".reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join(".reducers/reducer.js")).unwrap();
        f.write(b"function() {\
         if (this.counter == undefined) { \
           this.counter = 1;   \
         } else { \
           this.counter++;
         } \
         return {\"hello\": this.counter}; \
         }").unwrap();

        let issue = repo.new_issue().unwrap();

        // create two records
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();

        let state = issue.reduce_with_reducer(DuktapeReducer::new(&repo).unwrap()).unwrap();

        use serde_json::Number;
        assert_eq!(state.get("hello").unwrap(), &JsonValue::Number(Number::from(3)));
    }

    #[test]
    fn multiple_reducers() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join(".reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join(".reducers/reducer1.js")).unwrap();
        f.write(b"function(state) { return Object.assign({\"hello\": 1}, state); }").unwrap();
        let mut f = fs::File::create(repo.path().join(".reducers/reducer2.js")).unwrap();
        f.write(b"function(state) { return Object.assign({\"bye\": 2}, state); }").unwrap();

        let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(DuktapeReducer::new(&repo).unwrap()).unwrap();

        use serde_json::Number;
        assert_eq!(state.get("hello").unwrap(), &JsonValue::Number(Number::from(1)));
        assert_eq!(state.get("bye").unwrap(), &JsonValue::Number(Number::from(2)));
    }

    #[test]
    fn invalid_syntax() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join(".reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join(".reducers/reducer.js")).unwrap();
        f.write(b"function(state) { return Object.assign{\"hello\": 1}, state); }").unwrap();
        let res = DuktapeReducer::<::repository::Record>::new(&repo);
        assert!(res.is_err());
        let reducer_file = repo.path().join(".reducers/reducer.js");
        let err = res.unwrap_err();
        match err {
            Error::CompileError { file, error } => {
                assert_eq!(file, reducer_file);
                assert_eq!(error, "SyntaxError: unterminated statement (line 1)");
            },
            err => {
                panic!("Wrong type of error {}", err);
            }
        }
   }

    #[test]
    fn runtime_error() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join(".reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join(".reducers").join("reducer.js")).unwrap();
        f.write(b"function(state) { return Object.assign({\"hello\": record.a}, state); }").unwrap();

                let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(DuktapeReducer::new(&repo).unwrap()).unwrap();

        assert!(state.get("errors").is_some());
        let errors = state.get("errors").unwrap().as_array().unwrap();
        assert_eq!(errors[0].as_object().unwrap().get("error").unwrap(), &JsonValue::String("ReferenceError: identifier \'record\' undefined".into()));
        assert_eq!(errors[0].as_object().unwrap().get("file").unwrap(), &JsonValue::String(repo.path().join(".reducers").join("reducer.js").to_str().unwrap().into()));
    }

}
