use std::path::PathBuf;
use std::fs;
use std::ffi::OsString;
use fs_extra;
use sit_core::{Repository, Item, Record};
use pbr::ProgressBar;
use tempdir::TempDir;
use glob;

pub fn rebuild_repository<S: Into<PathBuf>>(src: S, dest: S, on_record: Option<S>) {
    let on_record: Option<OsString> = match on_record {
        Some(command) => {
            let path: PathBuf = command.into();
            Some(path.into_os_string())
        },
        None => None,
    };

    let src = Repository::open(src).expect("can't open source repository");
    let dest = Repository::new_with_config(dest, src.config().clone())
        .expect("can't create destination repository");
    // Copy all files and directories except for `config` and `items`
    print!("Copying all supplementary files: ");
    let dir = fs::read_dir(src.path()).expect("can't read source repository record");
    dir.filter(Result::is_ok)
        .map(Result::unwrap)
        .filter(|f| {
            let file_name = f.file_name();
            let name = file_name.to_str().unwrap();
            name != "config.json" &&
                name != "items"
        })
        .for_each(|f| {
            let file_name = f.file_name();
            let name = file_name.to_str().unwrap();
            let typ = f.file_type().expect(&format!("can't get file type for {}", name));
            if typ.is_file() {
                let copy_options = fs_extra::file::CopyOptions::new();
                fs_extra::file::copy(f.path(), dest.path().join(name), &copy_options)
                    .expect(&format!("can't copy file {}", name));
            } else if typ.is_dir() {
                let mut copy_options = fs_extra::dir::CopyOptions::new();
                copy_options.copy_inside = true;
                fs_extra::dir::copy(f.path(), dest.path().join(name), &copy_options)
                    .expect(&format!("can't copy directory {}", name));
            }
        });
    println!("done");

    // Process items
    let item_count = src.item_iter().expect("can't iterate over source repository's items")
        .count();

    println!("Processing items");

    let mut pb = ProgressBar::new(item_count as u64);
    for item in src.item_iter().expect("can't iterate over source repository's items") {
        let dest_item = dest.new_named_item(item.id())
            .expect("can't create an item in the destination repository");
        use std::collections::HashMap;
        let mut renames = HashMap::new();
        pb.inc();
        let recs = item.record_iter()
            .expect(&format!("can't iterate through records of {}", item.id()));
        for records in recs {
            for record in records {
                let tmp = TempDir::new("sit").expect("can't create temp directory");
                for (name, _reader) in record.file_iter() {
                    let path = src.items_path().join(item.id()).join(record.encoded_hash())
                        .join(&name);
                    let p = PathBuf::from(&name);
                    if p.components().count() > 1 {
                        let mut dir = p.clone();
                        dir.pop();
                        let dir = tmp.path().join(&dir);
                        fs::create_dir_all(&dir).expect(&format!("can't create directory {:?}", dir));
                    }

                    if name.starts_with(".prev/") {
                        // if there's a reference to a previous hash, it must have
                        // been recorded in `renames` already
                        let hash = &name[6..];
                        let new_prev = renames.get(hash).unwrap();
                        fs::File::create(tmp.path().join(".prev").join(new_prev))
                            .expect("can't create a new reference to a previous record");
                    } else {
                        let copy_options = fs_extra::file::CopyOptions::new();
                        fs_extra::file::copy(path, tmp.path().join(p), &copy_options)
                            .expect(&format!("can't copy file {}", name));
                    }

                }

                match on_record {
                    Some(ref command) => {
                        ::std::process::Command::new(command)
                            .arg(tmp.path().to_str().unwrap())
                            .current_dir(tmp.path())
                            .status()
                            .expect("can't execute on-record hook");
                    },
                    None => (),
                }

                let new_files =
                glob::glob(&format!("{}/**/*", tmp.path().to_str().unwrap()))
                    .expect("invalid glob pattern")
                    .filter(Result::is_ok)
                    .map(Result::unwrap)
                    .filter(|f| f.is_file())
                    .map(|f| {
                        let f1 = f.clone();
                        let name = f1.strip_prefix(tmp.path()).unwrap().to_str().unwrap();
                        (String::from(name), fs::File::open(f).expect("can't open file"))
                    });

                let new_record = dest_item.new_record(new_files, false).expect("can't create a record in destination repository");
                renames.insert(record.encoded_hash(), new_record.encoded_hash());
            }
        }
    }
    pb.finish();
}
