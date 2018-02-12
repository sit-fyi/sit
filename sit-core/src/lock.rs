//! Provides `Lock` trait

/// Primitive lock trait
pub trait Lock {
    /// Unlocks the lock
    fn unlock(self);
}

use std::io;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use fs2::FileExt;

/// File-based lock
pub struct FileLock(PathBuf, File);

impl FileLock {
    /// Returns a new lock
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, io::Error>  {
        let file = File::create(path.as_ref())?;
        file.lock_exclusive()?;
        Ok(FileLock(path.as_ref().into(), file))
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

impl Lock for FileLock {
    fn unlock(self) {
        let _ = self.1.unlock();
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use tempdir::TempDir;

    #[test]
    fn lock_cleanup() {
        let tmp = TempDir::new("sit").unwrap();
        let path = tmp.path().join("test_lock");
        let lock = FileLock::new(&path).unwrap();
        assert!(path.is_file());
        lock.unlock();
        assert!(!path.is_file());
    }


    #[test]
    fn lock_drop_cleanup() {
        let tmp = TempDir::new("sit").unwrap();
        let path = tmp.path().join("test_lock");
        let lock = FileLock::new(&path).unwrap();
        assert!(path.is_file());
        drop(lock);
        assert!(!path.is_file());
    }

    #[test]
    fn lock_wait() {
        use std::thread;
        let tmp = TempDir::new("sit").unwrap();
        let path = tmp.path().join("test_lock");
        let lock = FileLock::new(&path).unwrap();
        assert!(path.is_file());
        let path_clone = path.clone();
        let thread = thread::spawn(move || {
            FileLock::new(path_clone).unwrap()
        });
        thread::sleep(::std::time::Duration::from_millis(200));
        // unlock the first lock
        thread::spawn(move || {
            lock.unlock();
        });
        // this should be reachable
        let lock2 = thread.join().unwrap();
        lock2.unlock();
        assert!(!path.is_file());
    }

    #[test]
    fn lock_drop_wait() {
        use std::thread;
        let tmp = TempDir::new("sit").unwrap();
        let path = tmp.path().join("test_lock");
        let lock = FileLock::new(&path).unwrap();
        assert!(path.is_file());
        let path_clone = path.clone();
        let thread = thread::spawn(move || {
            FileLock::new(path_clone).unwrap()
        });
        thread::sleep(::std::time::Duration::from_millis(200));
        // drop the first lock
        thread::spawn(move || {
            drop(lock)
        });
        // this should be reachable
        let lock2 = thread.join().unwrap();
        lock2.unlock();
        assert!(!path.is_file());
    }

}