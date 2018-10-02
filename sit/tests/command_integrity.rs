extern crate cli_test_dir;
extern crate sit_core;

use sit_core::{Repository, path::HasPath};
#[cfg(feature = "deprecated-items")]
use sit_core::path::ResolvePath;
use cli_test_dir::*;

/// Should list records with failed integrity check
#[test]
#[cfg(feature = "deprecated-items")]
fn integrity_failure_item() {
    let dir = TestDir::new("sit", "integrity_failure_item");
    dir.cmd()
        .arg("init")
        .expect_success();
    // create an item
    let id = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap();
    // create a record
    let record = String::from_utf8(dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", id.trim(), "--no-author", "-t", "Sometype"])
        .expect_success().stdout).unwrap();
    // at this point, integrity check should not fail
    dir.cmd().arg("integrity").expect_success();
    // now, lets tamper with the record
    dir.create_file(dir.path(".sit/items").join(id.trim()).join(record.trim()).resolve_dir("/").unwrap().join("tamper"), "");
    // now, integrity check should fail
    // (we event set -i/--disable-integrity-check to make sure the command works with integrity check
    //  suppressed from the command line)
    let output = String::from_utf8(dir.cmd().args(&["-i", "integrity"]).expect_failure().stdout).unwrap();
    assert_eq!(output, format!("{} {}\n", id.trim(), record.trim()));
}

/// Should list records with failed integrity check
#[test]
fn integrity_failure() {
    let dir = TestDir::new("sit", "integrity_failure");
    dir.cmd()
        .arg("init")
        .expect_success();
    // create a record
    let record = String::from_utf8(dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", "--no-author", "-t", "Sometype"])
        .expect_success().stdout).unwrap();
    // at this point, integrity check should not fail
    dir.cmd().arg("integrity").expect_success();
    // now, lets tamper with the record
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let rec = repo.record(record.trim()).unwrap();
    dir.create_file(rec.path().join("tamper"), "");
    // now, integrity check should fail
    // (we event set -i/--disable-integrity-check to make sure the command works with integrity check
    //  suppressed from the command line)
    let output = String::from_utf8(dir.cmd().args(&["-i", "integrity"]).expect_failure().stdout).unwrap();
    assert_eq!(output, format!("{}\n", record.trim()));
}


/// Should not list records with failed integrity check unless it is disabled
#[test]
#[cfg(feature = "deprecated-items")]
fn integrity_check_flag_item() {
    let dir = TestDir::new("sit", "integrity_pass_item");
    dir.cmd()
        .arg("init")
        .expect_success();
    // create an item
    let id = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap();
    // create a record
    let record = String::from_utf8(dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", id.trim(), "--no-author", "-t", "Sometype"])
        .expect_success().stdout).unwrap();
    // now, lets tamper with the record
    dir.create_file(dir.path(".sit/items").join(id.trim()).join(record.trim()).resolve_dir("/").unwrap().join("tamper"), "");
    // now, the record should not appear
    let output = String::from_utf8(dir.cmd().args(&["records", id.trim()]).expect_success().stdout).unwrap();
    assert_eq!(output, "");
    // but if we disable integrity check:
    let output = String::from_utf8(dir.cmd().args(&["-i", "records", id.trim()]).expect_success().stdout).unwrap();
    assert_eq!(output, format!("{}\n", record.trim()));
    // or if we disable it through env:
     let output = String::from_utf8(dir.cmd().env("SIT_DISABLE_INTEGRITY_CHECK", "1").args(&["records", id.trim()]).expect_success().stdout).unwrap();
    assert_eq!(output, format!("{}\n", record.trim()));
}

/// Should not list records with failed integrity check unless it is disabled
#[test]
fn integrity_check_flag() {
    let dir = TestDir::new("sit", "integrity_pass");
    dir.cmd()
        .arg("init")
        .expect_success();
    // create a record
    let record = String::from_utf8(dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", "--no-author", "-t", "Sometype"])
        .expect_success().stdout).unwrap();
    // now, lets tamper with the record
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let rec = repo.record(record.trim()).unwrap();
    dir.create_file(rec.path().join("tamper"), "");
    // now, the record should not appear
    let output = String::from_utf8(dir.cmd().args(&["records"]).expect_success().stdout).unwrap();
    assert_eq!(output, "");
    // but if we disable integrity check:
    let output = String::from_utf8(dir.cmd().args(&["-i", "records"]).expect_success().stdout).unwrap();
    assert_eq!(output, format!("{}\n", record.trim()));
    // or if we disable it through env:
     let output = String::from_utf8(dir.cmd().env("SIT_DISABLE_INTEGRITY_CHECK", "1").args(&["records"]).expect_success().stdout).unwrap();
    assert_eq!(output, format!("{}\n", record.trim()));
}
