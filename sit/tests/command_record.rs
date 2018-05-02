extern crate cli_test_dir;
extern crate sit_core;
extern crate git2;
extern crate chrono;
extern crate which;

use cli_test_dir::*;
use sit_core::{Repository, Item, record::RecordExt};
use std::process;

/// Should derive authorship from the config file
#[test]
fn record_authorship() {
    let dir = TestDir::new("sit", "record_no_authorship_local_git");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", r#"{"author": {"name": "Test", "email": "test@test.com"}}"#);
    #[cfg(windows)]
    dir.create_file("sit_config.json", r#"{"author": {"name": "Test", "email": "test@test.com"}}"#);
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", &id, "-t", "Sometype"])
        .expect_success();
    verify_authors(&dir, &id,"Test <test@test.com>");
}


/// Should fail if there's no authorship configured and no git configs available
#[test]
fn record_no_authorship_no_git() {
    let dir = TestDir::new("sit", "record_no_authorship_no_git");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = dir.cmd()
        .arg("item")
        .expect_success().stdout;
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", ::std::str::from_utf8(&id).unwrap(), "-t","Sometype"])
        .expect_failure();
}

/// Should not attempt to record record authorship if specifically asked to do so
#[test]
fn record_no_authorship_no_author() {
    let dir = TestDir::new("sit", "record_no_authorship_no_author");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", &id, "--no-author", "-t", "Sometype"])
        .expect_success();
}

/// Should derive authorship from /working/directory/.git/config if it is otherwise unavailable
#[test]
fn record_no_authorship_local_git() {
    let dir = TestDir::new("sit", "record_no_authorship_local_git");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.create_file(".git/config", "[user]\nname=Test\nemail=test@test.com");
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", &id, "-t", "Sometype"])
        .expect_success();
    verify_authors(&dir, &id,"Test <test@test.com>");
}

/// Should derive authorship from $HOME/.gitconfig if it is otherwise unavailable
#[test]
fn record_no_authorship_user_git() {
    let dir = TestDir::new("sit", "record_no_authorship_user_git");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.create_file(".gitconfig","[user]\nname=Test\nemail=test@test.com");
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are right configs
        .args(&["record", &id, "-t","Sometype"])
        .expect_success();
    verify_authors(&dir, &id,"Test <test@test.com>");
}

/// Should prefer .git/config over $HOME/.gitconfig if authorship information is unavailable otherwise
#[test]
fn record_no_authorship_local_over_user_git() {
    let dir = TestDir::new("sit", "record_no_authorship_local_over_user_git");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.create_file(".gitconfig","[user]\nname=Test\nemail=test@test.com");
    dir.create_file(".git/config","[user]\nname=User\nemail=user@test.com");
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are right configs
        .args(&["record", &id, "-t","Sometype"])
        .expect_success();
    verify_authors(&dir, &id,"User <user@test.com>");
}

/// Should record a timestamp
#[test]
fn record_should_record_timestamp() {
    let dir = TestDir::new("sit", "record_should_record_timestamp");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are right configs
        .args(&["record", &id, "--no-author", "-t","Sometype"])
        .expect_success();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id).unwrap();
    let mut records = item.record_iter().unwrap();
    let record = records.next().unwrap().pop().unwrap();
    let mut s = String::new();
    use std::io::Read;
    record.file(".timestamp").unwrap().read_to_string(&mut s).unwrap();
    use chrono::prelude::*;
    let date = DateTime::parse_from_rfc3339(&s).unwrap();
    let now = Utc::now();
    assert_eq!(now.signed_duration_since(date).num_seconds(), 0);
    assert!(now.signed_duration_since(date).num_milliseconds() > 0);
}

/// Should not record a timestamp if asked to do so
#[test]
fn record_should_not_record_timestamp() {
    let dir = TestDir::new("sit", "record_should_not_record_timestamp");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", &id, "--no-author", "--no-timestamp", "-t","Sometype"])
        .expect_success();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id).unwrap();
    let mut records = item.record_iter().unwrap();
    let record = records.next().unwrap().pop().unwrap();
    assert!(record.file(".timestamp").is_none());
}

/// Should fail if any file to be recorded does not exist
#[test]
fn record_should_not_record_if_files_are_missing() {
    let dir = TestDir::new("sit", "record_should_not_record_if_files_are_missing");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.create_file("exists","");
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", &id, "--no-author", "-t","Sometype", "exists", "missing"])
        .expect_failure();
}

/// Should fail if no type is supplied
#[test]
fn record_should_fail_if_no_type() {
     let dir = TestDir::new("sit", "record_should_fail_if_no_type");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.create_file("file", "");
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", &id, "--no-author", "file"])
        .expect_failure();
}

/// Should not require -t if .type/... is supplied
#[test]
fn record_dot_type_sufficiency() {
    let dir = TestDir::new("sit", "record_dot_type_sufficiency");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.create_file(".type/MyType","");
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .args(&["record", &id, "--no-author", ".type/MyType"])
        .expect_success();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id).unwrap();
    let mut records = item.record_iter().unwrap();
    let record = records.next().unwrap().pop().unwrap();
    assert!(record.file(".type/MyType").is_some());
}


/// Should merge types from files and -t
#[test]
fn record_should_merge_types() {
    let dir = TestDir::new("sit", "record_should_merge_types");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.create_file(".type/MyType","");
    dir.create_file(".type/OurType","");
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are right configs
        .args(&["record", &id, "--no-author", "-t","Sometype,SomeOtherType",".type/MyType", ".type/OurType"])
        .expect_success();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id).unwrap();
    let mut records = item.record_iter().unwrap();
    let record = records.next().unwrap().pop().unwrap();
    assert!(record.file(".type/Sometype").is_some());
    assert!(record.file(".type/SomeOtherType").is_some());
    assert!(record.file(".type/MyType").is_some());
    assert!(record.file(".type/OurType").is_some());
}

/// Should record files
#[test]
fn record_should_record_files() {
    let dir = TestDir::new("sit", "record_should_record_files");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();
    dir.create_file("file1","file1");
    dir.create_file("files/file2","file2");
    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are right configs
        .args(&["record", &id, "--no-author", "-t","Sometype","file1", "files/file2"])
        .expect_success();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id).unwrap();
    let mut records = item.record_iter().unwrap();
    let record = records.next().unwrap().pop().unwrap();
    let mut s = String::new();
    use std::io::Read;
    record.file("file1").unwrap().read_to_string(&mut s).unwrap();
    assert_eq!(s, "file1");
    s.clear();
    record.file("files/file2").unwrap().read_to_string(&mut s).unwrap();
    assert_eq!(s, "file2");
}

/// Should sign if configuration says so
#[test]
fn record_should_sign_if_configured() {
    #[cfg(unix)]
    let dir = TestDir::new("sit", "record_should_sign_if_configured");
    #[cfg(windows)] // workaround for "File name too long" error
    let dir = TestDir::new("sit", "rssic");

    let gpg = which::which("gpg").expect("should have gpg installed");

    let mut genkey = process::Command::new(&gpg)
        .args(&["--batch", "--generate-key","-"])
        .env("GNUPGHOME", dir.path(".").to_str().unwrap())
        .stdin(::std::process::Stdio::piped())
        .stdout(::std::process::Stdio::null())
        .stderr(::std::process::Stdio::null())
        .spawn().unwrap();

    {
        use std::io::Write;
        let stdin = genkey.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(r#"
        Key-Type: default
        Subkey-Type: default
        Name-Real: Test
        Name-Comment: Test
        Name-Email: test@test.com
        Expire-Date: 0
        %no-protection
        %commit
        "#.as_bytes()).expect("Failed to write to stdin");
    }
    genkey.expect_success();

    #[cfg(unix)]
    let cfg = ".config/sit/config.json";
    #[cfg(windows)]
    let cfg = "sit_config.json";
    dir.create_file(cfg, r#"{"author": {"name": "Test", "email": "test@test.com"}, "signing": {"enabled": true, "key": "test@test.com"}}"#);

    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();

    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are right configs
        .env("GNUPGHOME", dir.path(".").to_str().unwrap())
        .args(&["record", &id, "--no-author", "-t","Sometype"])
        .expect_success();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id).unwrap();
    let mut records = item.record_iter().unwrap();
    let record = records.next().unwrap().pop().unwrap();
    assert!(record.file(".signature").is_some());
}

/// Should sign if instructed via command line
#[test]
fn record_should_sign_if_instructed_cmdline() {
    #[cfg(unix)]
    let dir = TestDir::new("sit", "record_should_sign_if_instructed_cmdline");
    #[cfg(windows)] // workaround for "File name too long" error
    let dir = TestDir::new("sit", "rssiic");

    let gpg = which::which("gpg").expect("should have gpg installed");

    let mut genkey = process::Command::new(&gpg)
        .args(&["--batch", "--generate-key","-"])
        .env("GNUPGHOME", dir.path(".").to_str().unwrap())
        .stdin(::std::process::Stdio::piped())
        .stdout(::std::process::Stdio::null())
        .stderr(::std::process::Stdio::null())
        .spawn().unwrap();

    {
        use std::io::Write;
        let stdin = genkey.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(r#"
        Key-Type: default
        Subkey-Type: default
        Name-Real: Test
        Name-Comment: Test
        Name-Email: test@test.com
        Expire-Date: 0
        %no-protection
        %commit
        "#.as_bytes()).expect("Failed to write to stdin");
    }
    genkey.expect_success();

    dir.cmd()
        .arg("init")
        .expect_success();
    let id: String = String::from_utf8(dir.cmd()
        .arg("item")
        .expect_success().stdout).unwrap().trim().into();

    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are right configs
        .env("GNUPGHOME", dir.path(".").to_str().unwrap())
        .args(&["record", "--sign",  "--signing-key", "test@test.com", &id, "--no-author", "-t","Sometype"])
        .expect_success();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id).unwrap();
    let mut records = item.record_iter().unwrap();
    let record = records.next().unwrap().pop().unwrap();
    assert!(record.file(".signature").is_some());
}


fn verify_authors<S0: AsRef<str>, S: AsRef<str>>(dir: &TestDir, id: S0, expected: S) {
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id).unwrap();
    let mut records = item.record_iter().unwrap();
    let record = records.next().unwrap().pop().unwrap();
    let mut s = String::new();
    use std::io::Read;
    record.file(".authors").unwrap().read_to_string(&mut s).unwrap();
    assert_eq!(s, expected.as_ref());
}

