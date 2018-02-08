extern crate cc;
extern crate include_dir;

use std::env;
use std::path::Path;
use include_dir::include_dir;

fn main() {

    match env::var("CARGO_FEATURE_DUKTAPE") {
        Ok(ref flag) if flag == "1" => {
                cc::Build::new()
                    .file("src/duktape/duktape.c")
                    .compile("duktape");

                println!("cargo:rustc-link-lib=static=duktape");
            },
        _ => (),
    }

    let outdir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&outdir).join("default_files.rs");
    include_dir(Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap())
                    .join("default-files").to_str().unwrap())
        .as_variable("FILES")
        .to_file(dest_path)
        .unwrap();

}