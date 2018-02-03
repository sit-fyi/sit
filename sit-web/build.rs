extern crate include_dir;

use std::env;
use std::path::Path;
use include_dir::include_dir;

fn main() {
    let outdir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&outdir).join("assets.rs");
    include_dir(Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("webapp").to_str().unwrap())
        .as_variable("FILES")
        .to_file(dest_path)
        .unwrap();
}