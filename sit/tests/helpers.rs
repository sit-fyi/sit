extern crate cli_test_dir;

use cli_test_dir::TestDir;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[allow(unused_variables)]
pub fn create_script(dir: &TestDir, unix_path: &str, windows_path: &str, unix: &str, windows: &str) {
    #[cfg(unix)] {
        dir.create_file(unix_path, unix);
        let mut perms = fs::metadata(dir.path(unix_path)).unwrap().permissions();
        perms.set_mode(0o766);
        fs::set_permissions(dir.path(unix_path), perms).unwrap();
    }
    #[cfg(windows)]
    dir.create_file(windows_path, windows);
}


