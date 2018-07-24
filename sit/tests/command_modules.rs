extern crate cli_test_dir;
extern crate sit_core;
extern crate dunce;
use sit_core::serde_json::Value;

use cli_test_dir::*;
use std::fs;
use sit_core::Repository;

mod helpers;
use helpers::*;

/// Tests that modules defined via <repo>/modules/<directory> convention are listed by `sit modules`
#[test]
fn modules_convention_dir() {
    let dir = TestDir::new("sit", "modules_convention_dir");
    dir.cmd()
        .arg("init")
        .expect_success();

    fs::create_dir_all(dir.path(".sit/modules/test")).unwrap();

    let output = String::from_utf8(dir.cmd().arg("modules").expect_success().stdout).unwrap();
    assert_eq!(output.trim(), dunce::canonicalize(dir.path(".sit/modules/test")).unwrap().to_str().unwrap());
}

/// Tests that modules defined via <repo>/modules/<file> convention are listed by `sit modules`
#[test]
fn modules_convention_link() {
    let dir = TestDir::new("sit", "modules_convention_link");
    dir.cmd()
        .arg("init")
        .expect_success();

    fs::create_dir_all(dir.path("module")).unwrap();
    dir.create_file(".sit/modules/module", "../../module");

    let output = String::from_utf8(dir.cmd().arg("modules").expect_success().stdout).unwrap();
    assert_eq!(output.trim(), dunce::canonicalize(dir.path("module")).unwrap().to_str().unwrap());
}

/// Tests that modules defined via external module manager are listed by `sit modules`
#[test]
fn modules_convention_ext() {
    let dir = TestDir::new("sit", "modules_convention_ext");
    dir.cmd()
        .arg("init")
        .expect_success();

    let mut repo = Repository::open(dir.path(".sit")).unwrap();
    repo.config_mut().set_extra_properties(vec![("external_module_manager", Value::String("modman".into()))]);
    repo.save().unwrap();

    create_script(&dir, ".sit/cli/sit-modman", ".sit/cli/sit-modman.bat",
                  r#"#! /usr/bin/env bash
                  echo /some/module
                  "#,
                  r#"
                  @echo /some/module
                  "#);

    let output = String::from_utf8(dir.cmd().arg("modules").expect_success().stdout).unwrap();
    assert_eq!(output.trim(), "/some/module");
}

/// Tests how `sit modules` will fail if the module manager is not available
#[test]
fn modules_convention_ext_invalid() {
    let dir = TestDir::new("sit", "modules_convention_ext");
    dir.cmd()
        .arg("init")
        .expect_success();

    let mut repo = Repository::open(dir.path(".sit")).unwrap();
    repo.config_mut().set_extra_properties(vec![("external_module_manager", Value::String("modman".into()))]);
    repo.save().unwrap();

    let output = String::from_utf8(dir.cmd().arg("modules").expect_failure().stderr).unwrap();
    assert!(output.contains("Can't find external module manager `sit-modman`"));
}
