extern crate cli_test_dir;
extern crate sit_core;
extern crate serde_json;

use sit_core::{Repository, Item};

use cli_test_dir::*;

/// Should fail if there is no item to reduce
#[test]
fn no_item() {
    let dir = TestDir::new("sit", "no_item");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.cmd().args(&["reduce", "some-item"]).expect_failure();
}

/// Should return the entire reduced item
#[test]
fn item() {
    let dir = TestDir::new("sit", "item");
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
    Repository::open(dir.path(".sit")).unwrap().item(id.trim()).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["reduce", id.trim()]).expect_success().stdout).unwrap();
    use serde_json::Map;
    let mut expect = Map::new();
    expect.insert("id".into(), serde_json::Value::String(id.trim().into()));
    expect.insert("value".into(), serde_json::Value::String("hello".into()));
    assert_eq!(serde_json::from_str::<serde_json::Value>(output.trim()).unwrap(), serde_json::Value::Object(expect));

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
    Repository::open(dir.path(".sit")).unwrap().item(id.trim()).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["reduce", id.trim(), "-q", "join(' ', ['item', id, value])"]).expect_success().stdout).unwrap();
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
    Repository::open(dir.path(".sit")).unwrap().item(id.trim()).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    dir.create_file(".sit/.items/queries/q1", "join(' ', ['item', id, value])");
    let output = String::from_utf8(dir.cmd().args(&["reduce", id.trim(), "-Q", "q1"]).expect_success().stdout).unwrap();
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
    Repository::open(dir.path(".sit")).unwrap().item(id.trim()).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    let cfg = r#"{"items": {"queries": {"q1": "join(' ', ['item', id, value])"}}}"#;
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(windows)]
    dir.create_file("sit_config.json", cfg);
    let output = String::from_utf8(dir.cmd().env("HOME", dir.path(".").to_str().unwrap()).args(&["reduce", id.trim(), "-Q", "q1"]).expect_success().stdout).unwrap();
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
    Repository::open(dir.path(".sit")).unwrap().item(id.trim()).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    let cfg = r#"{"items": {"queries": {"q1": "join(' ', ['item', id])"}}}"#;
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(windows)]
    dir.create_file("sit_config.json", cfg);
    dir.create_file(".sit/.items/queries/q1", "join(' ', ['item', id, value])");
    let output = String::from_utf8(dir.cmd().env("HOME", dir.path(".").to_str().unwrap()).args(&["reduce", id.trim(), "-Q", "q1"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), format!("item {} hello", id.trim()));
}
