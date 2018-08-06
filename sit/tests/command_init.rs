extern crate cli_test_dir;
extern crate sit_core;

use cli_test_dir::*;
use sit_core::Repository;
use std::fs;

/// Should initialize a repository
#[test]
fn repo_init() {
    let dir = TestDir::new("sit", "repo_init");
    dir.cmd()
        .arg("init")
        .expect_success();
    assert!(dir.path(".sit").is_dir());
    assert!(Repository::open(dir.path(".sit")).is_ok());
}

/// Should initialize a repository in an empty directory (absolute)
#[test]
fn repo_init_emptydir_absolute() {
    let dir = TestDir::new("sit", "repo_init_emptydir_absolute");
    dir.cmd()
        .args(&["-r", dir.path(".").to_str().unwrap()])
        .arg("init")
        .expect_success();
    assert!(dir.path("config.json").is_file());
    assert!(Repository::open(dir.path(".")).is_ok());
}

/// Should initialize a repository in an empty directory (relative)
#[test]
fn repo_init_emptydir_relative() {
    let dir = TestDir::new("sit", "repo_init_emptydir_relative");
    dir.cmd()
        .args(&["-r", "."])
        .arg("init")
        .expect_success();
    assert!(dir.path("config.json").is_file());
    assert!(Repository::open(dir.path(".")).is_ok());
}

/// Should return failing status when unable to initialize a repository
#[test]
fn repo_init_fail() {
    let dir = TestDir::new("sit", "repo_init_fail");
    let path = "test";
    dir.create_file(path, "can't have a directory here");
    dir.cmd()
        .arg("-r")
        .arg(path)
        .arg("init")
        .expect_failure();
}

/// Should keep existing repository as is
#[test]
fn repo_reinit() {
    let dir = TestDir::new("sit", "repo_reinit");
    dir.cmd()
        .arg("init")
        .expect_success();
    assert!(dir.path(".sit").is_dir());
    let repo = Repository::open(dir.path(".sit")).unwrap();
    assert_eq!(repo.item_iter().unwrap().count(), 0);
    repo.new_item().unwrap();
    assert_eq!(repo.item_iter().unwrap().count(), 1);
    dir.cmd()
        .arg("init")
        .expect_success();
    let repo = Repository::open(dir.path(".sit")).unwrap();
    // still has that one item
    assert_eq!(repo.item_iter().unwrap().count(), 1);
}

/// Should respect working directory
#[test]
fn repo_init_workdir() {
    let dir = TestDir::new("sit", "repo_init_workdir");
    fs::create_dir_all(dir.path("workdir")).unwrap();
    dir.cmd()
        .arg("-d")
        .arg(dir.path("workdir"))
        .arg("init")
        .expect_success();
    assert!(dir.path("workdir").join(".sit").is_dir());
    assert!(Repository::open(dir.path("workdir").join(".sit")).is_ok());
}

/// Should respect repository path
#[test]
fn repo_init_repo_path() {
    let dir = TestDir::new("sit", "repo_init_repo_path");
    dir.cmd()
        .arg("-r")
        .arg(dir.path("repo"))
        .arg("init")
        .expect_success();
    assert!(dir.path("repo").is_dir());
    assert!(Repository::open(dir.path("repo")).is_ok());
}

/// Should concatenate working directory and a repository if both are supplied
#[test]
fn repo_init_path_concat() {
    let dir = TestDir::new("sit", "repo_init_path_concat");
    fs::create_dir_all(dir.path("workdir")).unwrap();
    dir.cmd()
        .arg("-d")
        .arg(dir.path("workdir"))
        .arg("-r")
        .arg("repo")
        .arg("init")
        .expect_success();
    assert!(dir.path("workdir").join("repo").is_dir());
    assert!(Repository::open(dir.path("workdir").join("repo")).is_ok());
}
