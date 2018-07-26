fn user_config(dir: &TestDir, cfg: &str) {
    #[cfg(all(unix, not(target_os = "macos")))]
    dir.create_file(".config/sit/config.json", cfg);
    #[cfg(all(unix, target_os = "macos"))]
    dir.create_file("Library/Preferences/fyi.sit.sit/config.json", cfg);
    #[cfg(windows)] {
      dir.create_file("AppData/Roaming/sit/sit/config/config.json", cfg);
      // Make sure required directories exist
      ::std::fs::create_dir_all(dir.path("AppData/Local")).unwrap();
    }
}

#[allow(unused_variables, dead_code)]
fn no_user_config(dir: &TestDir) {
    #[cfg(windows)] {
      // Make sure required directories exist
      ::std::fs::create_dir_all(dir.path("AppData/Local")).unwrap();
      ::std::fs::create_dir_all(dir.path("AppData/Roaming/sit/sit/config")).unwrap();
    }
}
