extern crate cli_test_dir;
extern crate sit_core;

use cli_test_dir::*;

/// Should list no items if there are none
#[test]
fn no_items() {
    let dir = TestDir::new("sit", "no_items");
    dir.cmd()
        .arg("init")
        .expect_success();
    let output = String::from_utf8(dir.cmd().arg("items").expect_success().stdout).unwrap();
    assert_eq!(output, "");
}

/// Should list an item if there's one
#[test]
fn item() {
    let dir = TestDir::new("sit", "item");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let output = String::from_utf8(dir.cmd().arg("items").expect_success().stdout).unwrap();
    assert_eq!(output, id);
}

/// Should apply filter
#[test]
fn item_filter() {
    let dir = TestDir::new("sit", "filter");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id_ = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    // filter out item we just created
    let output = String::from_utf8(dir.cmd().args(&["items","-f", &format!("id != '{}'", id.trim())]).expect_success().stdout).unwrap();
    assert_eq!(output, id_);
}

/// Should apply named filter
#[test]
fn item_named_filter() {
    let dir = TestDir::new("sit", "named_filter");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id_ = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    dir.create_file(".sit/.items/filters/f1", &format!("id != '{}'", id.trim()));
    // filter out item we just created
    let output = String::from_utf8(dir.cmd().args(&["items","-F", "f1"]).expect_success().stdout).unwrap();
    assert_eq!(output, id_);
}

/// Should apply named filter
#[test]
fn item_named_user_filter() {
    let dir = TestDir::new("sit", "named_user_filter");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id_ = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    // filter out item we just created
    let cfg = &format!(r#"{{"items": {{"filters": {{"f1": "id != '{}'"}}}}}}"#, id.trim());
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(windows)]
    dir.create_file("sit_config.json", cfg);
    let output = String::from_utf8(dir.cmd().env("HOME", dir.path(".").to_str().unwrap()).args(&["items","-F", "f1"]).expect_success().stdout).unwrap();
    assert_eq!(output, id_);
}

/// Should prefer repo named filter over user named filer
#[test]
fn item_repo_over_named_user_filter() {
    let dir = TestDir::new("sit", "named_repo_over_user_filter");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id_ = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let cfg = &format!(r#"{{"items": {{"filters": {{"f1": "id == '{}'"}}}}}}"#, id.trim());
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(windows)]
    dir.create_file("sit_config.json", cfg);
    // filter out item we just created
    dir.create_file(".sit/.items/filters/f1", &format!("id != '{}'", id.trim()));
    let output = String::from_utf8(dir.cmd().env("HOME", dir.path(".").to_str().unwrap()).args(&["items","-F", "f1"]).expect_success().stdout).unwrap();
    assert_eq!(output, id_);
}


/// Should apply query
#[test]
fn item_query() {
    let dir = TestDir::new("sit", "query");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    // create a record
    dir.cmd().args(&["record", "-t", "Test", id.trim()]).expect_success();
    let output = String::from_utf8(dir.cmd().args(&["items","-q", "join(' ', ['item', id, value])"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), format!("item {} hello", id.trim()));
}

/// Should apply named query
#[test]
fn item_named_query() {
    let dir = TestDir::new("sit", "named_query");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    // create a record
    dir.cmd().args(&["record", "-t", "Test", id.trim()]).expect_success();
    dir.create_file(".sit/.items/queries/q1", "join(' ', ['item', id, value])");
    let output = String::from_utf8(dir.cmd().args(&["items","-Q", "q1"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), format!("item {} hello", id.trim()));
}

/// Should apply named user query
#[test]
fn item_named_user_query() {
    let dir = TestDir::new("sit", "named_user_query");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    // create a record
    dir.cmd().args(&["record", "-t", "Test", id.trim()]).expect_success();
    let cfg = r#"{"items": {"queries": {"q1": "join(' ', ['item', id, value])"}}}"#;
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(windows)]
    dir.create_file("sit_config.json", cfg);
    let output = String::from_utf8(dir.cmd().env("HOME", dir.path(".").to_str().unwrap()).args(&["items","-Q", "q1"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), format!("item {} hello", id.trim()));
}

/// Should prefer repo named query over user user named query
#[test]
fn item_repo_over_named_user_query() {
    let dir = TestDir::new("sit", "repo_over_named_user_query");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    // create a record
    dir.cmd().args(&["record", "-t", "Test", id.trim()]).expect_success();
    let cfg = r#"{"items": {"queries": {"q1": "join(' ', ['item', id])"}}}"#;
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(windows)]
    dir.create_file("sit_config.json", cfg);
    dir.create_file(".sit/.items/queries/q1", "join(' ', ['item', id, value])");
    let output = String::from_utf8(dir.cmd().env("HOME", dir.path(".").to_str().unwrap()).args(&["items","-Q", "q1"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), format!("item {} hello", id.trim()));
}

