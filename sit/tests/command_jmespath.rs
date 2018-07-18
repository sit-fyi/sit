extern crate cli_test_dir;
extern crate serde_json;

use cli_test_dir::*;
use std::process::Stdio;
use std::io::Write;

#[test]
fn jmespath() {
    let dir = TestDir::new("sit", "jmespath");
    let mut child = dir.cmd()
        .args(&["jmespath", "merge(@, {\"processed\": `true`})"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn().unwrap();

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all("{\"test\": 1}".as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("failed to read stdout");
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.get("test").unwrap().as_i64().unwrap(), 1);
    assert_eq!(obj.get("processed").unwrap().as_bool().unwrap(), true);
}

#[test]
fn jmespath_pretty() {
    let dir = TestDir::new("sit", "jmespath_pretty");
    let mut child = dir.cmd()
        .args(&["jmespath", "merge(@, {\"processed\": `true`})", "--pretty"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn().unwrap();

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all("{\"test\": 1}".as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("failed to read stdout");
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.get("test").unwrap().as_i64().unwrap(), 1);
    assert_eq!(obj.get("processed").unwrap().as_bool().unwrap(), true);
}
