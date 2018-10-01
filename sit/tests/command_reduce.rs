extern crate cli_test_dir;
extern crate sit_core;
extern crate serde_json;

use sit_core::{Repository, record::RecordOwningContainer, Record};

use cli_test_dir::*;

include!("includes/config.rs");

/// Should fail if there is no item to reduce
#[test]
#[cfg(feature = "deprecated-items")]
fn no_item() {
    let dir = TestDir::new("sit", "no_item");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.cmd().args(&["reduce", "some-item"]).expect_failure();
}

/// Should return the entire reduced item
#[test]
#[cfg(feature = "deprecated-items")]
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

/// Should return the entire reduced repository
#[test]
fn reduce_repo() {
    let dir = TestDir::new("sit", "reduce_repo");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    // create a record
    Repository::open(dir.path(".sit")).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["reduce"]).expect_success().stdout).unwrap();
    use serde_json::Map;
    let mut expect = Map::new();
    expect.insert("value".into(), serde_json::Value::String("hello".into()));
    assert_eq!(serde_json::from_str::<serde_json::Value>(output.trim()).unwrap(), serde_json::Value::Object(expect));
}

/// Should return the entire reduced repository
/// using a custom reducer
#[test]
fn reduce_repo_custom() {
    let dir = TestDir::new("sit", "reduce_repo_custom");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file("test.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    // create a record
    Repository::open(dir.path(".sit")).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["reduce", "-r", dir.path("test.js").to_str().unwrap()]).expect_success().stdout).unwrap();
    use serde_json::Map;
    let mut expect = Map::new();
    expect.insert("value".into(), serde_json::Value::String("hello".into()));
    assert_eq!(serde_json::from_str::<serde_json::Value>(output.trim()).unwrap(), serde_json::Value::Object(expect));
}

/// Should return the entire reduced repository
/// using a named reducer
#[test]
fn reduce_repo_named() {
    let dir = TestDir::new("sit", "reduce_repo_named");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test/reducer.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    // create a record
    Repository::open(dir.path(".sit")).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["reduce", "-r", "test"]).expect_success().stdout).unwrap();
    use serde_json::Map;
    let mut expect = Map::new();
    expect.insert("value".into(), serde_json::Value::String("hello".into()));
    assert_eq!(serde_json::from_str::<serde_json::Value>(output.trim()).unwrap(), serde_json::Value::Object(expect));
}




/// Should apply query
#[test]
fn record_query() {
    let dir = TestDir::new("sit", "query");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    // create a record
    Repository::open(dir.path(".sit")).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["reduce", "-q", "join(' ', ['item', value])"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "item hello");
}

/// Should apply named query
#[test]
fn record_named_query() {
    let dir = TestDir::new("sit", "named_query");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    // create a record
    Repository::open(dir.path(".sit")).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    dir.create_file(".sit/.queries/q1", "join(' ', ['item', value])");
    let output = String::from_utf8(dir.cmd().args(&["reduce", "-Q", "q1"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "item hello");
}

/// Should apply named user query
#[test]
fn record_named_user_query() {
    let dir = TestDir::new("sit", "named_user_query");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    // create a record
    Repository::open(dir.path(".sit")).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    let cfg = r#"{"items": {"queries": {"q1": "join(' ', ['item', value])"}}}"#;
    user_config(&dir, cfg);
    let output = String::from_utf8(dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap())
        .env("USERPROFILE", dir.path(".").to_str().unwrap())
        .args(&["reduce", "-Q", "q1"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "item hello");
}

/// Should prefer repo named query over user user named query
#[test]
fn record_repo_over_named_user_query() {
    let dir = TestDir::new("sit", "repo_over_named_user_query");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        return Object.assign(state, {value: "hello"});
    }
    "#);
    // create a record
    Repository::open(dir.path(".sit")).unwrap().new_record(vec![("test", &b""[..])].into_iter(), true).unwrap();
    let cfg = r#"{"items": {"queries": {"q1": "join(' ', ['item'])"}}}"#;
    user_config(&dir, cfg);
    dir.create_file(".sit/.queries/q1", "join(' ', ['item', value])");
    let output = String::from_utf8(dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap())
        .env("USERPROFILE", dir.path(".").to_str().unwrap())
        .args(&["reduce", "-Q", "q1"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "item hello");
}

/// Should reduce starting at a fixed root
#[test]
fn reduce_repo_fixed_roots() {
    let dir = TestDir::new("sit", "reduce_repo_fixed_roots");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        var v = state.value || "";
        v = v + new TextDecoder('utf-8').decode(record.files.test);
        return Object.assign(state, {value: v});
    }
    "#);
    // create a record
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let _rec1 = repo.new_record(vec![("test", &b"1"[..])].into_iter(), false).unwrap();
    let rec2 = repo.new_record(vec![("test", &b"2"[..])].into_iter(), false).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["reduce", "--root", &rec2.encoded_hash()]).expect_success().stdout).unwrap();
    use serde_json::Map;
    let mut expect = Map::new();
    expect.insert("value".into(), serde_json::Value::String("2".into()));
    assert_eq!(serde_json::from_str::<serde_json::Value>(output.trim()).unwrap(), serde_json::Value::Object(expect));
}

/// Should reduce starting with a certain state
#[test]
fn reduce_repo_initial_state() {
    let dir = TestDir::new("sit", "reduce_repo_initial_state");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.create_file(".sit/reducers/test.js",r#"
    module.exports = function(state, record) {
        var v = state.value || "";
        v = v + new TextDecoder('utf-8').decode(record.files.test);
        return Object.assign(state, {value: v});
    }
    "#);
    // create a record
    let repo = Repository::open(dir.path(".sit")).unwrap();
    repo.new_record(vec![("test", &b"1"[..])].into_iter(), false).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["reduce", "--state", "{\"value\": \"0\"}"]).expect_success().stdout).unwrap();
    use serde_json::Map;
    let mut expect = Map::new();
    expect.insert("value".into(), serde_json::Value::String("01".into()));
    assert_eq!(serde_json::from_str::<serde_json::Value>(output.trim()).unwrap(), serde_json::Value::Object(expect));
}
