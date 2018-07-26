extern crate cli_test_dir;

use cli_test_dir::*;
use std::fs;

/// `sit path` should print repository's directory
#[test]
fn path() {
    let dir = TestDir::new("sit", "path");
    dir.cmd()
        .arg("init")
        .expect_success();
    let path = String::from_utf8(dir.cmd().arg("path").expect_success().stdout).unwrap();
    assert_eq!(path.trim(), dir.path(".sit").to_str().unwrap());
}

/// `sit path` should print repository's directory when it's suplied over SIT_DIR
#[test]
fn path_sit_dir() {
    let dir = TestDir::new("sit", "path_sit_dir");
    fs::create_dir_all(dir.path("1")).unwrap();
    fs::create_dir_all(dir.path("2")).unwrap();
    dir.cmd()
        .current_dir(dir.path("1"))
        .arg("init")
        .expect_success();
    dir.cmd()
        .current_dir(dir.path("2"))
        .arg("init")
        .expect_success();
    let path = String::from_utf8(dir.cmd().current_dir(dir.path("2"))
        .env("SIT_DIR", dir.path("1").join(".sit")).arg("path")
        .expect_success().stdout).unwrap();
    assert_eq!(path.trim(), dir.path("1").join(".sit").to_str().unwrap());
}

