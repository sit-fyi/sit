extern crate cli_test_dir;

use cli_test_dir::*;

include!("includes/config.rs");

/// Should fail if there's no authorship configured and no git configs available
#[test]
fn web_no_authorship_no_git() {
    let dir = TestDir::new("sit", "web_no_authorship_no_git");
    dir.cmd()
        .arg("init")
        .expect_success();
    no_user_config(&dir);
    let out = dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .env("USERPROFILE", dir.path(".").to_str().unwrap())
        .args(&["web"])
        .expect_failure().stderr;
    assert!(String::from_utf8(out).unwrap().contains("SIT needs your authorship identity to be configured"));
}

/// Should derive authorship from git if it is available
#[test]
fn web_no_authorship() {
    let dir = TestDir::new("sit", "web_no_authorship");
    dir.cmd()
        .arg("init")
        .expect_success();
    no_user_config(&dir);
    dir.create_file(".git/config", "[user]\nname=Test\nemail=test@test.com");
    let out = dir.cmd()
        .env("HOME", dir.path(".").to_str().unwrap()) // to ensure there are no configs
        .env("USERPROFILE", dir.path(".").to_str().unwrap())
        .args(&["web","-"])
        .expect_failure().stderr;
    // should fail because the socket address is invalid, but not the authorship
    println!("{}", String::from_utf8(out.clone()).unwrap());
    assert!(String::from_utf8(out).unwrap().contains("invalid socket address"));
}
