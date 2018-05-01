extern crate cli_test_dir;
extern crate sit_core;

use cli_test_dir::*;
use sit_core::{Repository, Item};

/// Should create an item
#[test]
fn item() {
    let dir = TestDir::new("sit", "item");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let mut items = repo.item_iter().unwrap();
    let item = items.next().unwrap();
    assert_eq!(item.id(), id.trim());
    assert!(items.next().is_none());
}

/// Should create a named item
#[test]
fn item_named() {
    let dir = TestDir::new("sit", "item_named");
    dir.cmd()
        .arg("init")
       .expect_success();
    let id = String::from_utf8(dir.cmd().args(&["item", "--id", "test"]).expect_success().stdout).unwrap();
    assert_eq!(id.trim(), "test");
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let mut items = repo.item_iter().unwrap();
    let item = items.next().unwrap();
    assert_eq!(item.id(), id.trim());
    assert!(items.next().is_none());
}

/// Should fail if creating a named item with a duplicate name
/// (item with such name already exists)
#[test]
fn item_existing() {
    let dir = TestDir::new("sit", "existing");
    dir.cmd()
        .arg("init")
       .expect_success();
    dir.cmd().args(&["item", "--id", "test"]).expect_success();
    let err = String::from_utf8(dir.cmd().args(&["item", "--id", "test"]).expect_failure().stderr).unwrap();
    assert_eq!(err.trim(), "Item test already exists");
}
