use std::io::Read;

use super::Reducer;
use serde_json::{Map, Value as JsonValue};
use std::marker::PhantomData;
use ::Record;
use duktape;
use std::ptr;
use std::ffi::{CString, CStr, OsStr};
use std::path::PathBuf;
use std::fs;

#[cfg(feature = "duktape-mmap")]
use memmap;

#[cfg(feature = "cesu8")]
use cesu8;

#[derive(Debug)]
pub struct DuktapeReducer<'a, R: Record> {
    repository: &'a ::Repository,
    context: *mut duktape::duk_context,
    reducers: i32,
    filenames: Vec<PathBuf>,
    phantom_data: PhantomData<R>,
    functions: Vec<Vec<u8>>,
}

unsafe impl<'a, R: Record> Send for DuktapeReducer<'a, R> {}

#[derive(Debug, Error)]
pub enum Error {
    IoError(::std::io::Error),
    #[error(no_from, non_std)]
    ExecutionError {
        error: String,
    },
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

    unsafe fn load_module(context: *mut duktape::duk_hthread) -> Result<(), Error> {
        // Now, execute the function with a defined module
        duktape::duk_require_function(context, -1);
        duktape::duk_push_object(context); // module
        duktape::duk_push_object(context); // module.exports
        let exports_prop = CString::new("exports").unwrap();
        duktape::duk_put_prop_string(context, -2, exports_prop.as_ptr());
        // f module
        duktape::duk_dup_top(context);
        duktape::duk_require_object(context, -1);
        duktape::duk_require_object(context, -2);
        duktape::duk_require_function(context, -3);
        // f module module
        duktape::duk_insert(context, -3);
        duktape::duk_require_object(context, -1);
        duktape::duk_require_function(context, -2);
        duktape::duk_require_object(context, -3);
        // module f module
        let res = duktape::duk_pcall(context,1);
        if res as u32 == duktape::DUK_EXEC_ERROR {
            let err_str = CStr::from_ptr(duktape::duk_to_string(context, -1));
            let error = err_str.to_str().unwrap().into();
            return Err(Error::ExecutionError { error });
        }
        // module retval
        duktape::duk_pop(context);
        // module
        duktape::duk_get_prop_string(context, -1, exports_prop.as_ptr());
        // module f'
        duktape::duk_remove(context, -2);
        // f'
        Ok(())
    }


    #[cfg(feature = "duktape-require")]
    unsafe extern "C" fn mod_search(context: *mut duktape::duk_hthread) -> duktape::duk_ret_t {
        // module id
        let id = CStr::from_ptr(duktape::duk_get_string(context, 0));
        // path to reducers
        let str_path_prop = CString::new("path").unwrap();
        let str_duktape = CString::new("Duktape").unwrap();
        duktape::duk_get_global_string(context, str_duktape.as_ptr());
        duktape::duk_get_prop_string(context, -1,str_path_prop.as_ptr());
        let path = CStr::from_ptr(duktape::duk_get_string(context, -1));
        duktape::duk_pop_2(context);
        // find the module
        let prefix = PathBuf::from(path.to_str().unwrap());
        let mod_path = prefix.join(id.to_str().unwrap());
        if mod_path.is_file() && mod_path.strip_prefix(&prefix).is_ok() {
            let mut f = fs::File::open(mod_path).unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            let src = CString::new(s).unwrap();
            duktape::duk_push_string(context, src.as_ptr());
            return 1;
        } else {
            let err = CString::new(format!("module not found: {:?}", id)).unwrap();
            duktape::duk_error_raw(context, duktape::DUK_ERR_ERROR as i32, ptr::null_mut(), 0,err.as_ptr());
            return duktape::DUK_RET_ERROR;
        }
    }

    pub fn new(repository: &'a ::Repository) -> Result<Self, Error> {
        let context = unsafe {
            duktape::duk_create_heap(None, None, None,ptr::null_mut(), Some(fatal_handler))
        };
        let reducers_path = repository.path().join("reducers");
        #[cfg(feature = "duktape-require")]
        unsafe {
            let str_duktape = CString::new("Duktape").unwrap();
            let str_mod_search = CString::new("modSearch").unwrap();
            let str_path = CString::new(reducers_path.to_str().unwrap()).unwrap();
            let str_path_prop = CString::new("path").unwrap();
            duktape::duk_module_duktape_init(context);
            duktape::duk_get_global_string(context, str_duktape.as_ptr());
            // path reducers
            duktape::duk_push_string(context, str_path_prop.as_ptr());
            duktape::duk_push_string(context, str_path.as_ptr());
            duktape::duk_def_prop(context, -3, duktape::DUK_DEFPROP_HAVE_VALUE);
            // function
            duktape::duk_push_c_function(context, Some(DuktapeReducer::<'a, R>::mod_search), 4);
            duktape::duk_put_prop_string(context, -2, str_mod_search.as_ptr());
           duktape::duk_pop(context);
        }
        let files = fs::read_dir(reducers_path).unwrap();
        let mut reducers = 0;
        let mut filenames = vec![];
        let mut functions = vec![];
        let files = files.filter(Result::is_ok).map(Result::unwrap).map(|e| e.path())
            .filter(|f| f.extension() == Some(OsStr::new("js")));
        // in test builds, we guarantee the order of files, but not in other builds as
        // it is not a great idea to rely on the order of these files
        #[cfg(test)]
        let mut files : Vec<_> = files.collect();
        #[cfg(test)]
        files.sort();
        for file in files {
            reducers += 1;
            unsafe {
                // source code
                let mut source = String::new();
                let mut f = fs::File::open(&file)?;
                let _ = f.read_to_string(&mut source)?;
                // prepare the module function
                source = format!("function (module) {{ {} }}", source);
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
                    // save bytecode
                    duktape::duk_dup_top(context);
                    duktape::duk_dump_function(context);
                    let mut sz = 0;
                    let data = duktape::duk_get_buffer(context, -1, &mut sz);
                    let mut func = vec![0; sz];
                    ptr::copy_nonoverlapping(data, func.as_mut_ptr() as *mut _, sz);
                    functions.push(func);
                    duktape::duk_pop(context);
                    // load module
                    DuktapeReducer::<'a, R>::load_module(context)?;
                    // If module.export is not function, bail
                    if duktape::duk_is_function(context, -1) != 1 {
                        return Err(Error::CompileError {
                            file,
                            error: "module.exports should export a function".into(),
                        })
                    }
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
            functions,
            phantom_data: PhantomData,
        })
    }

    /// Resets every reducer's state back to an empty object
    ///
    /// Very useful for re-using the same set of reducers for
    /// multiple issues, helps avoiding re-reading and re-compiling
    /// reducer functions every time.
    pub fn reset_state(&self) {
        for i in 0..self.reducers {
            unsafe {
                duktape::duk_push_object(self.context);
                duktape::duk_swap_top(self.context,i * 2 + 1);
                duktape::duk_pop(self.context);
            }
        }
    }


}

impl<'a, R: Record> Clone for DuktapeReducer<'a, R> {
    fn clone(&self) -> Self {
        let context = unsafe {
            duktape::duk_create_heap(None, None, None,ptr::null_mut(), Some(fatal_handler))
        };

        unsafe {
            for (i, func) in self.functions.iter().enumerate() {
                // load bytecode
                duktape::duk_push_buffer_raw(context, 0, duktape::DUK_BUF_FLAG_DYNAMIC | duktape::DUK_BUF_FLAG_EXTERNAL);
                duktape::duk_config_buffer(context, -1, func.as_ptr() as *mut _, func.len());
                duktape::duk_load_function(context);
                // obtain the module
                DuktapeReducer::<'a, R>::load_module(context).unwrap(); // since it's a clone we assume the first load went fine
                // transfer state
                duktape::duk_push_null(self.context);
                duktape::duk_copy(self.context, (i * 2 + 1) as i32, -1);
                duktape::duk_json_encode(self.context, -1);
                let state = duktape::duk_get_string(self.context, -1);
                duktape::duk_pop(self.context);
                duktape::duk_push_string(context, state);
                duktape::duk_json_decode(context, -1);
            }
        }
        DuktapeReducer {
            repository: self.repository,
            context,
            reducers: self.reducers,
            filenames: self.filenames.clone(),
            functions: self.functions.clone(),
            phantom_data: PhantomData,
        }
    }
}


impl<'a, R: Record> Reducer for DuktapeReducer<'a, R> {
    type State = Map<String, JsonValue>;
    type Item = R;

    fn reduce(&mut self, mut state: Self::State, item: &Self::Item) -> Self::State {
        use serde_json;

        let json = serde_json::to_string(&JsonValue::Object(state.clone())).unwrap();
        unsafe {
            let ctx = self.context;

            #[cfg(feature = "cesu8")]
            let json_cstring = CString::new(cesu8::to_cesu8(&json)).unwrap();

            #[cfg(not(feature = "cesu8"))]
            let json_cstring = CString::new(json).unwrap();

            // Item (record)
            duktape::duk_push_object(ctx);
            // item.hash
            let hash = CString::new(item.encoded_hash().as_ref()).unwrap();
            duktape::duk_push_string(ctx, hash.as_ptr());
            let hash_prop = CString::new("hash").unwrap();
            duktape::duk_put_prop_string(ctx, -2, hash_prop.as_ptr());
            // item.files
            duktape::duk_push_object(ctx);
            #[cfg(feature = "duktape-mmap")]
            let mut mmaps = vec![];
            for (name, mut reader) in item.file_iter() {
                let filename = CString::new(name.as_ref()).unwrap();
                #[cfg(feature = "duktape-mmap")] {
                    // avoid unused warning
                    let _ = reader;
                    #[cfg(windows)] // replace slashes with backslashes
                    let name = name.as_ref().replace("/", "\\");
                    #[cfg(not(windows))]
                    let name = name.as_ref();

                    let path = self.repository.issues_path()
                        .join(item.issue_id().as_ref())
                        .join(item.encoded_hash().as_ref())
                        .join(name);

                    if fs::metadata(&path).unwrap().len() == 0 {
                        // if the file is empty, it can't be mmapped
                        // (also, no reason to do so anyway)
                        duktape::duk_push_buffer_raw(ctx, 0, duktape::DUK_BUF_MODE_FIXED);
                    } else {
                        let file = fs::File::open(&path).unwrap();
                        let mmap = memmap::MmapOptions::new().map(&file).unwrap();
                        duktape::duk_push_buffer_raw(ctx, 0, duktape::DUK_BUF_FLAG_DYNAMIC | duktape::DUK_BUF_FLAG_EXTERNAL);
                        mmaps.push(mmap);
                        let mmap_ref = &mmaps[mmaps.len() - 1];
                        duktape::duk_config_buffer(ctx, -1, mmap_ref.as_ptr() as *mut _, mmap_ref.len());
                    }
                }
                #[cfg(not(feature = "duktape-mmap"))] {
                    use std::io::Read;
                    // INEFFICIENT BUT WORKS FOR NOW {
                    let mut buf = vec![];
                    let sz = reader.read_to_end(&mut buf).unwrap();
                    let ptr = duktape::duk_push_buffer_raw(ctx,sz, 0);
                    ptr::copy_nonoverlapping(buf.as_ptr(), ptr.offset(0) as *mut _, sz);
                    // }
                }
                duktape::duk_put_prop_string(ctx, -2, filename.as_ptr());
            }
            let files_prop = CString::new("files").unwrap();
            duktape::duk_put_prop_string(ctx, -2, files_prop.as_ptr());


            // Current issue state
            duktape::duk_push_string(ctx, json_cstring.as_ptr());
            duktape::duk_json_decode(ctx, -1);

            for i in 0..self.reducers {
                // function
                duktape::duk_require_function(ctx, i * 2);
                duktape::duk_dup(ctx, i * 2);
                // reducer's state
                duktape::duk_require_object(ctx,i * 2 + 1);
                duktape::duk_dup(ctx, i * 2 + 1);
                // issue state
                duktape::duk_push_null(ctx);
                // save previous state
                duktape::duk_copy(ctx, -4, -1);
                duktape::duk_require_object(ctx, -1);
                // item (record)
                duktape::duk_dup(ctx, -5);
                duktape::duk_require_object(ctx, -1);

                // execute
                let res = duktape::duk_pcall_method(ctx,2);

               // now, check for error
                if res as u32 == duktape::DUK_EXEC_ERROR {
                    let err = ::std::ffi::CStr::from_ptr(duktape::duk_safe_to_lstring(ctx, -1, ptr::null_mut()));
                    {
                        let mut arr = state.entry(String::from("errors")).or_insert(JsonValue::Array(vec![]));
                        let mut error = Map::new();
                        error.insert("file".into(), JsonValue::String(self.filenames[i as usize].to_str().unwrap().into()));
                        error.insert("error".into(), JsonValue::String(err.to_str().unwrap().into()));
                        arr.as_array_mut().unwrap().push(JsonValue::Object(error));
                    }
                    return state;
                }

                if duktape::duk_is_object(ctx, -1) == 1 {
                    // drop extra state
                    duktape::duk_swap_top(ctx, -2);
                    duktape::duk_pop(ctx);
                    // now it should be [item, current issue state] again
                } else if duktape::duk_is_undefined(ctx, -1) == 1 {
                    // restore previous state
                    duktape::duk_copy(ctx, -2, -1);
                } else {
                    let err = format!("TypeError: invalid return value {}, expected an object", ::std::ffi::CStr::from_ptr(duktape::duk_safe_to_lstring(ctx, -1, ptr::null_mut())).to_string_lossy());
                    {
                        let mut arr = state.entry(String::from("errors")).or_insert(JsonValue::Array(vec![]));
                        let mut error = Map::new();
                        error.insert("file".into(), JsonValue::String(self.filenames[i as usize].to_str().unwrap().into()));
                        error.insert("error".into(), JsonValue::String(err));
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
            #[cfg(feature = "cesu8")]
            let map: Map<String, JsonValue> = match cesu8::from_cesu8(json.to_bytes()) {
                Ok(s) => serde_json::from_str(&s),
                Err(_) => serde_json::from_slice(json.to_bytes()),
            }.unwrap();
            #[cfg(not(feature = "cesu8"))]
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
    fn undefined_result() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/2.js")).unwrap();
        f.write(b"module.exports = function(state, record) {  }").unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/1.js")).unwrap();
        f.write(b"module.exports = function(state, record) { return {test: true} }").unwrap();

        let issue = repo.new_issue().unwrap();
        let _record = issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

        assert_eq!(state.get("test").unwrap(), &JsonValue::Bool(true));
    }

    #[test]
    fn mistyped_result() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = function(state, record) { return 1 }").unwrap();

        let issue = repo.new_issue().unwrap();
        let _record = issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

            assert!(state.get("errors").is_some());
        let errors = state.get("errors").unwrap().as_array().unwrap();
        assert_eq!(errors[0].as_object().unwrap().get("error").unwrap(), &JsonValue::String("TypeError: invalid return value 1, expected an object".into()));
        assert_eq!(errors[0].as_object().unwrap().get("file").unwrap(), &JsonValue::String(repo.path().join("reducers").join("reducer.js").to_str().unwrap().into()));
    }


    #[test]
    fn record_hash() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = function(state, record) { return {\"hello\": record.hash}; }").unwrap();

        let issue = repo.new_issue().unwrap();
        let record = issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

        assert_eq!(state.get("hello").unwrap(), &JsonValue::String(record.encoded_hash()));
    }

    #[test]
    fn record_contents() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = function(state, record) { return {\"hello\": new TextDecoder('utf-8').decode(record.files.text)}; }").unwrap();

        let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

        assert_eq!(state.get("hello").unwrap(), &JsonValue::String("Title".into()));
    }


    #[test]
    fn reducer_state() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = function() {\
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

        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

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
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer1.js")).unwrap();
        f.write(b"module.exports = function(state) { return Object.assign({\"hello\": 1}, state); }").unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer2.js")).unwrap();
        f.write(b"module.exports = function(state) { return Object.assign({\"bye\": 2}, state); }").unwrap();

        let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

        use serde_json::Number;
        assert_eq!(state.get("hello").unwrap(), &JsonValue::Number(Number::from(1)));
        assert_eq!(state.get("bye").unwrap(), &JsonValue::Number(Number::from(2)));
    }

    #[test]
    fn module_export_non_function_error() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = 'hello'").unwrap();
        let res = DuktapeReducer::<::repository::Record>::new(&repo);
        assert!(res.is_err());
        let reducer_file = repo.path().join("reducers/reducer.js");
        let err = res.unwrap_err();
        match err {
            Error::CompileError { file, error } => {
                assert_eq!(file, reducer_file);
                assert_eq!(error, "module.exports should export a function");
            },
            err => {
                panic!("Wrong type of error {}", err);
            }
        }
    }

    #[test]
    fn module_export_props() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer1.js")).unwrap();
        f.write(b"module.exports = function(state) { return Object.assign({\"hello\": module.exports.data}, state); }; module.exports.data = 1;").unwrap();

        let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

        use serde_json::Number;
        assert_eq!(state.get("hello").unwrap(), &JsonValue::Number(Number::from(1)));
    }

    #[test]
    fn module_closure() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer1.js")).unwrap();
        f.write(b"var a = 1; module.exports = function(state) { return Object.assign({\"hello\": a}, state); };").unwrap();

        let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

        use serde_json::Number;
        assert_eq!(state.get("hello").unwrap(), &JsonValue::Number(Number::from(1)));
    }


    #[test]
    fn anonymous_function_error() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"function(state) { return state }").unwrap();
        let res = DuktapeReducer::<::repository::Record>::new(&repo);
        assert!(res.is_err());
        let reducer_file = repo.path().join("reducers/reducer.js");
        let err = res.unwrap_err();
        match err {
            Error::CompileError { file, error } => {
                assert_eq!(file, reducer_file);
                assert_eq!(error, "SyntaxError: function name required (line 1)");
            },
            err => {
                panic!("Wrong type of error {}", err);
            }
        }
    }

    #[test]
    fn invalid_syntax() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = function(state) { return Object.assign{\"hello\": 1}, state); }").unwrap();
        let res = DuktapeReducer::<::repository::Record>::new(&repo);
        assert!(res.is_err());
        let reducer_file = repo.path().join("reducers/reducer.js");
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
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers").join("reducer.js")).unwrap();
        f.write(b"module.exports = function(state) { return Object.assign({\"hello\": record.a}, state); }").unwrap();

        let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

        assert!(state.get("errors").is_some());
        let errors = state.get("errors").unwrap().as_array().unwrap();
        assert_eq!(errors[0].as_object().unwrap().get("error").unwrap(), &JsonValue::String("ReferenceError: identifier \'record\' undefined".into()));
        assert_eq!(errors[0].as_object().unwrap().get("file").unwrap(), &JsonValue::String(repo.path().join("reducers").join("reducer.js").to_str().unwrap().into()));
    }

    #[test]
    fn resetting_state() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = function() {\
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

        let mut reducer = DuktapeReducer::new(&repo).unwrap();

        use serde_json::Number;

        let state = issue.reduce_with_reducer(&mut reducer).unwrap();
        assert_eq!(state.get("hello").unwrap(), &JsonValue::Number(Number::from(3)));

        // run it again without touching the state
        let state = issue.reduce_with_reducer(&mut reducer).unwrap();
        assert_eq!(state.get("hello").unwrap(), &JsonValue::Number(Number::from(6)));

        // now, reset state
        reducer.reset_state();

        let state = issue.reduce_with_reducer(&mut reducer).unwrap();
        assert_eq!(state.get("hello").unwrap(), &JsonValue::Number(Number::from(3)));
    }

    #[test]
    fn cloned() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"\
        var a = 1;\
        module.exports = function() {\
         if (this.counter == undefined) { \
           this.counter = a;   \
         } else { \
           this.counter++;
         } \
         return {\"hello\": this.counter}; \
         };").unwrap();

        let issue1 = repo.new_issue().unwrap();
        issue1.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();

        let issue2 = repo.new_issue().unwrap();
        issue2.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        issue2.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title 1"[..])].into_iter(), true).unwrap();


        let reducer = DuktapeReducer::new(&repo).unwrap();
        let mut reducer1 = reducer.clone();
        let mut reducer2 = reducer.clone();

        let state1 = issue1.reduce_with_reducer(&mut reducer1).unwrap();
        let state2 = issue2.reduce_with_reducer(&mut reducer2).unwrap();

        use serde_json::Number;
        assert_eq!(state1.get("hello").unwrap(), &JsonValue::Number(Number::from(1)));
        assert_eq!(state2.get("hello").unwrap(), &JsonValue::Number(Number::from(2)));

        // Now, make sure state gets copied from where it is, and not the original value:
        let mut reducer3 = reducer2.clone();
        let state3 = issue1.reduce_with_reducer(&mut reducer3).unwrap();
        assert_eq!(state3.get("hello").unwrap(), &JsonValue::Number(Number::from(3)));
    }


    // Duktape uses CESU-8 internally, which is not the standard UTF-8
    // encoding. Make sure we convert whatever is produced by Duktape.
    #[cfg(feature = "cesu8")]
    #[test]
    fn cesu8_output() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = function(state, record) { \
            return Object.assign(state, {hello: new TextDecoder('utf-8').decode(record.files.text)});
        }").unwrap();

        let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &"🙂😵😾🤔".as_bytes()[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

        assert_eq!(state.get("hello").unwrap(), &JsonValue::String("🙂😵😾🤔".into()));
    }

    #[cfg(feature = "cesu8")]
    #[test]
    fn cesu8_input() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();

        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = function(state, record) {
          if (typeof record.files['.type/DetailsChanged'] !== 'undefined') {
              var decoder = new TextDecoder('utf-8');
              return Object.assign(state, {
                  details: decoder.decode(record.files.text).trim()
              });
          } else {
              return state;
          }
        }").unwrap();

        let mut f = fs::File::create(repo.path().join("reducers/reducer1.js")).unwrap();
        f.write(b"module.exports = function(state, record) {
          if (typeof record.files['.type/Commented'] !== 'undefined') {
            var comments = this.comments || [];
            var decoder = new TextDecoder('utf-8');
            comments.push({
               text: decoder.decode(record.files.text),
            });
            this.comments = comments;
            return Object.assign(state, {comments: comments});
          }
          return state;
        }").unwrap();

        let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/DetailsChanged", &b""[..]), ("text", &"🙂😵😾🤔   ".as_bytes()[..])].into_iter(), true).unwrap();
        issue.new_record(vec![(".type/Commented", &b""[..]), ("text", &"test🙂😵".as_bytes()[..])].into_iter(), true).unwrap();

        // SHOULD NOT FAIL
        issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();
    }

    #[cfg(feature = "duktape-require")]
    #[test]
    fn require() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers").join("reducer")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = require(\"reducer/index.js\");").unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer/index.js")).unwrap();
        f.write(b"module.exports = function(state, record) { return {\"hello\": record.hash}; }").unwrap();

        let issue = repo.new_issue().unwrap();
        let record = issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(&mut DuktapeReducer::new(&repo).unwrap()).unwrap();

        assert_eq!(state.get("hello").unwrap(), &JsonValue::String(record.encoded_hash()));
    }

    #[cfg(feature = "duktape-require")]
    #[test]
    fn require_path_modification() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers").join("reducer")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"Duktape.path = Duktape.path + '/reducer'; module.exports = require(\"index.js\");").unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer/index.js")).unwrap();
        f.write(b"module.exports = function(state, record) { return {\"hello\": record.hash}; }").unwrap();

        let err_str = "Error: module not found: \"index.js\"";
        assert_matches!(DuktapeReducer::<::repository::Record>::new(&repo),
        Err(Error::ExecutionError { ref error }) if error == err_str);
    }


    #[cfg(feature = "duktape-require")]
    #[test]
    fn require_not_found() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers").join("reducer")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = require(\"reducer/index.js\");").unwrap();

        let err_str = "Error: module not found: \"reducer/index.js\"";
        assert_matches!(DuktapeReducer::<::repository::Record>::new(&repo),
        Err(Error::ExecutionError { ref error }) if error == err_str);
    }


    #[cfg(feature = "duktape-require")]
    #[test]
    fn require_external_relative() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = require(\"../reducer.js\");").unwrap();

        let mut f = fs::File::create(repo.path().join("reducer.js")).unwrap();
        f.write(b"module.exports = function() {};").unwrap();

        let err_str = "TypeError: cannot resolve module id: ../reducer.js";
        assert_matches!(DuktapeReducer::<::repository::Record>::new(&repo),
        Err(Error::ExecutionError { ref error }) if error == err_str);
    }

    #[cfg(feature = "duktape-require")]
    #[test]
    fn require_external_absolute() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(repo.path().join("reducers")).unwrap();
        let mut f = fs::File::create(repo.path().join("reducers/reducer.js")).unwrap();
        f.write(b"module.exports = require(\"/reducer.js\");").unwrap();

        let err_str = "TypeError: cannot resolve module id: /reducer.js";
        assert_matches!(DuktapeReducer::<::repository::Record>::new(&repo),
        Err(Error::ExecutionError { ref error }) if error == err_str);
    }


}
