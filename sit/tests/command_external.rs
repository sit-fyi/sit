extern crate cli_test_dir;

use cli_test_dir::*;

mod helpers;
use helpers::*;

/// Should fail on an unavailable command
#[test]
fn ext_unavailable() {
    let dir = TestDir::new("sit", "ext_unavailable");
    dir.cmd()
        .arg("init")
        .expect_success();
    dir.cmd()
        .arg("does-not-exist")
        .expect_failure();
}

/// Should pass SIT variable pointing to SIT itself
#[test]
fn ext_sit_path() {
    let dir = TestDir::new("sit", "ext_cli");
    dir.cmd()
        .arg("init")
        .expect_success();
    create_script(&dir, ".sit/cli/sit-path", ".sit/cli/sit-path.bat", r#"#! /bin/bash
        echo -n ${SIT}
    "#, r#"
    @echo off
    echo %SIT%
    "#);

    let result = String::from_utf8(dir.cmd()
        .arg("path")
        .expect_success().stdout).unwrap();

    dir.expect_path(result.trim());
}

/// Should execute an external command available in .sit/cli
#[test]
fn ext_cli() {
    let dir = TestDir::new("sit", "ext_cli");
    dir.cmd()
        .arg("init")
        .expect_success();

    create_script(&dir, ".sit/cli/sit-exists", ".sit/cli/sit-exists.bat", r#"#! /bin/bash
        echo SIT_DIR=${SIT_DIR}
    "#, r#"
    @echo off
    echo SIT_DIR=%SIT_DIR%
    "#);

    let result = String::from_utf8(dir.cmd()
        .arg("exists")
        .expect_success().stdout).unwrap().replace("\r","");

    assert_eq!(result, format!("SIT_DIR={}\n", dir.path(".sit").to_str().unwrap()))
}

/// Should execute an external command available in .sit/modules/*/cli
#[test]
fn ext_modules_cli() {
    let dir = TestDir::new("sit", "ext_modules_cli");
    dir.cmd()
        .arg("init")
        .expect_success();

    create_script(&dir, ".sit/modules/test/cli/sit-exists", ".sit/modules/test/cli/sit-exists.bat", r#"#! /bin/bash
        echo SIT_DIR=${SIT_DIR}
    "#, r#"
    @echo off
    echo SIT_DIR=%SIT_DIR%
    "#);

    let result = String::from_utf8(dir.cmd()
        .arg("exists")
        .expect_success().stdout).unwrap().replace("\r","");

    assert_eq!(result, format!("SIT_DIR={}\n", dir.path(".sit").to_str().unwrap()))
}

/// Should execute an external command available in PATH
#[test]
fn ext_modules_path() {
    let dir = TestDir::new("sit", "ext_modules_path");
    dir.cmd()
        .arg("init")
        .expect_success();

    create_script(&dir, "sit-exists", "sit-exists.bat", r#"#! /bin/bash
        echo SIT_DIR=${SIT_DIR}
    "#, r#"
    @echo off
    echo SIT_DIR=%SIT_DIR%
    "#);

    let result = String::from_utf8(dir.cmd()
        .env("PATH", dir.path(".").to_str().unwrap())
        .arg("exists")
        .expect_success().stdout).unwrap().replace("\r","");

    assert_eq!(result, format!("SIT_DIR={}\n", dir.path(".sit").to_str().unwrap()))
}

/// Command in .sit/cli should take precedence over a command in .sit/modules/*/cli and PATH
#[test]
fn ext_cli_over_modules_cli_and_path() {
    let dir = TestDir::new("sit", "ext_cli_over_modules_cli_and_path");
    dir.cmd()
        .arg("init")
        .expect_success();

    create_script(&dir, "sit-exists", "sit-exists.bat", r#"#! /bin/bash
    echo path
    "#, r#"
    @echo off
    echo path
    "#);

    create_script(&dir, ".sit/cli/sit-exists", ".sit/cli/sit-exists.bat", r#"#! /bin/bash
    echo cli
    "#, r#"
    @echo off
    echo cli
    "#);

    create_script(&dir, ".sit/modules/test/cli/sit-exists", ".sit/modules/test/cli/sit-exists.bat", r#"#! /bin/bash
    echo modules
    "#, r#"
    @echo off
    echo modules
    "#);


    let result = String::from_utf8(dir.cmd()
        .env("PATH", dir.path(".").to_str().unwrap())
        .arg("exists")
        .expect_success().stdout).unwrap().replace("\r","");

    assert_eq!(result, "cli\n")
}

/// Command in .sit/modules/cli should take precedence over a command in PATH
#[test]
fn ext_modules_over_path() {
    let dir = TestDir::new("sit", "ext_modules_over_path");
    dir.cmd()
        .arg("init")
        .expect_success();

    create_script(&dir, "sit-exists", "sit-exists.bat", r#"#! /bin/bash
    echo path
    "#, r#"
    @echo off
    echo path
    "#);

    create_script(&dir, ".sit/modules/test/cli/sit-exists", ".sit/modules/test/cli/sit-exists.bat", r#"#! /bin/bash
    echo modules
    "#, r#"
    @echo off
    echo modules
    "#);


    let result = String::from_utf8(dir.cmd()
        .env("PATH", dir.path(".").to_str().unwrap())
        .arg("exists")
        .expect_success().stdout).unwrap().replace("\r","");

    assert_eq!(result, "modules\n")
}
