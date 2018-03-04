extern crate cc;
extern crate include_dir;

use std::env;
use std::path::Path;
use include_dir::include_dir;

fn main() {
    #[cfg(windows)] {
        use std::process::Command;
        let rustc = env::var("RUSTC").unwrap();
        let rustc_version = Command::new(rustc)
            .arg("--version")
            .output()
            .expect("can't run rustc --version");
        let version = rustc_version.stdout;
        if version.starts_with(b"rustc 1.24 (") {
            panic!("Rust 1.24 is known to break Windows builds. Please upgrade to 1.24.1+");
        }
    }
    match env::var("CARGO_FEATURE_DUKTAPE") {
        Ok(ref flag) if flag == "1" => {
            let mut build = cc::Build::new();
            build.file("src/duktape/duktape.c");
            if let Ok(_) = env::var("CARGO_FEATURE_DUKTAPE_REQUIRE") {
                build.file("src/duktape/duk_module_duktape.c");
            }
            if env::var("CARGO_FEATURE_WINDOWS7").is_ok() {
                build.define("DUK_USE_DATE_NOW_WINDOWS","1");
            }
            build.compile("duktape");

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