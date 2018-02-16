extern crate include_dir;
extern crate fs_extra;

use std::env;
use std::path::Path;
use include_dir::include_dir;


fn main() {
    let outdir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&outdir).join("assets.rs");
    let webapp = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("webapp");
    let mut doc = webapp.clone();
    doc.pop(); doc.pop();
    doc.push("doc");
    let mut opts = fs_extra::dir::CopyOptions::new();
    opts.copy_inside = true;
    let documentation = webapp.join("documentation");
    fs_extra::dir::remove(&documentation).expect("can't delete documentation");
    fs_extra::dir::copy(doc, &documentation, &opts).expect("can't copy documentation");
    include_dir(webapp.to_str().unwrap())
        .as_variable("FILES")
        .ignore("node_modules")
        .to_file(dest_path)
        .unwrap();
}