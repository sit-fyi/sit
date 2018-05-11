extern crate cli_test_dir;
extern crate sit_core;
extern crate which;
extern crate remove_dir_all;

use std::process;

use sit_core::{Repository, Item};

use cli_test_dir::*;
use remove_dir_all::*;

/// Should list no records if there are none
#[test]
fn no_records() {
    let dir = TestDir::new("sit", "no_records");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["records", id.trim()]).expect_success().stdout).unwrap();
    assert_eq!(output, "");
}


/// Should list a record if there's one
#[test]
fn record() {
    let dir = TestDir::new("sit", "rec_record");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let record = String::from_utf8(dir.cmd().args(&["record", id.trim(), "-t", "Type"]).expect_success().stdout).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["records", id.trim()]).expect_success().stdout).unwrap();
    assert_eq!(output, record);
}


/// Should apply filter
#[test]
fn filter() {
    let dir = TestDir::new("sit", "rec_filter");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let record = String::from_utf8(dir.cmd().args(&["record", id.trim(), "-t", "Type"]).expect_success().stdout).unwrap();
    let record1 = String::from_utf8(dir.cmd().args(&["record", id.trim(), "-t", "Type"]).expect_success().stdout).unwrap();
    // filter out item we just created
    let output = String::from_utf8(dir.cmd().args(&["records", id.trim(), "-f", &format!("hash != '{}'", record.trim())]).expect_success().stdout).unwrap();
    assert_eq!(output, record1);
}

/// Should apply filter
#[test]
fn named_filter() {
    let dir = TestDir::new("sit", "rec_named_filter");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let record = String::from_utf8(dir.cmd().args(&["record", id.trim(), "-t", "Type"]).expect_success().stdout).unwrap();
    let record1 = String::from_utf8(dir.cmd().args(&["record", id.trim(), "-t", "Type"]).expect_success().stdout).unwrap();
    // filter out item we just created
    dir.create_file(".sit/.records/filters/f1", &format!("hash != '{}'", record.trim()));
    let output = String::from_utf8(dir.cmd().args(&["records", id.trim(), "-F", "f1"]).expect_success().stdout).unwrap();
    assert_eq!(output, record1);
}


/// Should apply named user filter
#[test]
fn named_user_filter() {
        let dir = TestDir::new("sit", "rec_named_filter");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let record = String::from_utf8(dir.cmd().args(&["record", id.trim(), "-t", "Type"]).expect_success().stdout).unwrap();
    let record1 = String::from_utf8(dir.cmd().args(&["record", id.trim(), "-t", "Type"]).expect_success().stdout).unwrap();
    // filter out item we just created
    let cfg = &format!(r#"{{"records": {{"filters": {{"f1": "hash != '{}'"}}}}}}"#, record.trim());
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(windows)]
    dir.create_file("sit_config.json", cfg);
    let output = String::from_utf8(dir.cmd().env("HOME", dir.path(".").to_str().unwrap()).args(&["records", id.trim(), "-F", "f1"]).expect_success().stdout).unwrap();
    assert_eq!(output, record1);
}

/// Should prefer repo named filter over user named filer
#[test]
fn repo_over_named_user_filter() {
    let dir = TestDir::new("sit", "rec_named_repo_over_user_filter");
    dir.cmd()
        .arg("init")
        .expect_success();
       let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let record = String::from_utf8(dir.cmd().args(&["record", id.trim(), "-t", "Type"]).expect_success().stdout).unwrap();
    let record1 = String::from_utf8(dir.cmd().args(&["record", id.trim(), "-t", "Type"]).expect_success().stdout).unwrap();
    // filter out item we just created
    let cfg = &format!(r#"{{"records": {{"filters": {{"f1": "hash != '{}'"}}}}}}"#, record1.trim());
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(windows)]
    dir.create_file("sit_config.json", cfg);
    dir.create_file(".sit/.records/filters/f1", &format!("hash != '{}'", record1.trim()));
    let output = String::from_utf8(dir.cmd().env("HOME", dir.path(".").to_str().unwrap()).args(&["records", id.trim(), "-F", "f1"]).expect_success().stdout).unwrap();
    assert_eq!(output, record);
}

/// Should apply query
#[test]
fn query() {
    let dir = TestDir::new("sit", "rec_query");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id.trim()).unwrap();
    // create a record
    let _record = item.new_record(vec![("test", &b"passed"[..])].into_iter(), true).unwrap();
    let output = String::from_utf8(dir.cmd().args(&["records",id.trim(),"-q", "files.test"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "passed");
}


/// Should apply named query
#[test]
fn named_query() {
    let dir = TestDir::new("sit", "rec_named_query");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id.trim()).unwrap();
    // create a record
    let _record = item.new_record(vec![("test", &b"passed"[..])].into_iter(), true).unwrap();
    dir.create_file(".sit/.records/queries/q1", "files.test");
    let output = String::from_utf8(dir.cmd().args(&["records",id.trim(),"-Q", "q1"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "passed");
}


/// Should apply named user query
#[test]
fn named_user_query() {
    let dir = TestDir::new("sit", "rec_named_user_query");
    dir.cmd()
        .arg("init")
        .expect_success();
    let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id.trim()).unwrap();
    // create a record
    let _record = item.new_record(vec![("test", &b"passed"[..])].into_iter(), true).unwrap();
    let cfg = r#"{"records": {"queries": {"q1": "files.test"}}}"#;
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(windows)]
    dir.create_file("sit_config.json", cfg);
    let output = String::from_utf8(dir.cmd().env("HOME", dir.path(".").to_str().unwrap()).args(&["records", id.trim(), "-Q", "q1"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "passed");
}


/// Should prefer repo named query over user user named query
#[test]
fn repo_over_named_user_query() {
    let dir = TestDir::new("sit", "rec_repo_over_named_user_query");
    dir.cmd()
        .arg("init")
        .expect_success();
        let id = String::from_utf8(dir.cmd().arg("item").expect_success().stdout).unwrap();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    let item = repo.item(id.trim()).unwrap();
    // create a record
    let _record = item.new_record(vec![("test", &b"passed"[..])].into_iter(), true).unwrap();
    dir.create_file(".sit/.records/queries/q1", "files.test");
    let cfg = r#"{"records": {"queries": {"q1": "null"}}}"#;
    #[cfg(unix)]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(windows)]
    dir.create_file("sit_config.json", cfg);
    let output = String::from_utf8(dir.cmd().env("HOME", dir.path(".").to_str().unwrap()).args(&["records", id.trim(), "-Q", "q1"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "passed");
}

/// Should verify PGP signature if instructed
#[test]
fn pgp_signature() {
    let dir = TestDir::new("sit", "pgp");

    let gpg = which::which("gpg2").or_else(|_| which::which("gpg")).expect("should have gpg installed");

    let mut genkey = process::Command::new(&gpg)
        .args(&["--batch", "--gen-key","-"])
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

    let output = String::from_utf8(dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap())
        .env("GNUPGHOME", dir.path(".").to_str().unwrap())
        .args(&["records", id.trim(), "-v", "-q", "verification.success"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "true");
}

/// Should indicate if PGP signature is for something else
#[test]
fn pgp_signature_wrong_data() {
    let dir = TestDir::new("sit", "pgps");

    let gpg = which::which("gpg2").or_else(|_| which::which("gpg")).expect("should have gpg installed");

    let mut genkey = process::Command::new(&gpg)
        .args(&["--batch", "--gen-key","-"])
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

    // Snatch the signature
    let oldrec = String::from_utf8(dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are right configs
        .env("GNUPGHOME", dir.path(".").to_str().unwrap())
        .args(&["record", "--sign",  "--signing-key", "test@test.com", &id, "--no-author", "-t","Sometype"])
        .expect_success().stdout).unwrap();

    use std::fs::File;
    use std::io::{Read, Write};
    let mut f = File::open(dir.path(".sit").join("items").join(id.trim()).join(oldrec.trim()).join(".signature")).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    remove_dir_all(dir.path(".sit").join("items").join(id.trim()).join(oldrec.trim())).unwrap();

    let mut f = File::create(dir.path(".signature")).unwrap();
    f.write(s.as_bytes()).unwrap();
    //

    dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are right configs
        .env("GNUPGHOME", dir.path(".").to_str().unwrap())
        .args(&["record", &id, "--no-author", "-t","Sometype1", ".signature"])
        .expect_success();


    let output = String::from_utf8(dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap())
        .env("GNUPGHOME", dir.path(".").to_str().unwrap())
        .args(&["records", id.trim(), "-v", "-q", "verification.success"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "false");
}


/// Should not verify PGP key if there is no signature
#[test]
fn pgp_no_signature() {
    let dir = TestDir::new("sit", "pgpno");

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

    let output = String::from_utf8(dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap())
        .args(&["records", id.trim(), "-v", "-q", "verification"]).expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "null");
}
