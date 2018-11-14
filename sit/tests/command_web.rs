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


