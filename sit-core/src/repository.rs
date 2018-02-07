//! Repository is where SIT stores all of its artifacts.
//!
//! It is represented by the [`Repository`] structure.
//!
//! [`Repository`]: struct.Repository.html
//!


use std::path::{Component, Path, PathBuf};
use std::fs;

use glob;

use serde_json;

use super::hash::{HashingAlgorithm, Hasher};
use super::encoding::Encoding;
use super::id::IdGenerator;

/// Current repository format version
const VERSION: &str = "1";
/// Repository's config file name
const CONFIG_FILE: &str = "config.json";
/// Repository's issues path
const ISSUES_PATH: &str = "issues";

/// Repository is the container for all SIT artifacts
#[derive(Debug)]
pub struct Repository {
    /// Path to the container
    path: PathBuf,
    /// Path to the config file. Mainly to avoid creating
    /// this path on demand for every operation that would
    /// require it
    config_path: PathBuf,
    /// Path to issues. Mainly to avoid creating this path
    /// on demand for every operation that would require it
    issues_path: PathBuf,
    /// Configuration
    config: Config,
}

/// Repository configuration
#[derive(Debug, Clone, TypedBuilder, Serialize, Deserialize)]
pub struct Config {
     /// Hashing algorithm used
    hashing_algorithm: HashingAlgorithm,
    /// Encoding used
    encoding: Encoding,
    /// ID generator
    id_generator: IdGenerator,
    /// Repository version
    #[default = "String::from(VERSION)"]
    version: String,
}

#[derive(Debug, Error)]
pub enum Error {
    /// Item already exists
    AlreadyExists,
    /// Item not found
    NotFound,
    /// Invalid repository version
    #[error(no_from, non_std)]
    InvalidVersion {
        expected: String,
        got: String,
    },
    /// I/O error
    IoError(::std::io::Error),
    /// JSON (de)serialization error
    SerializationError(serde_json::Error),
    /// Base decoding error
    BaseDecodeError(::data_encoding::DecodeError),
}

impl Repository {

    /// Attempts creating a new repository. Fails with `Error::AlreadyExists`
    /// if a repository already exists.
    pub fn new<P: Into<PathBuf>>(path: P) -> Result<Self, Error> {
        Repository::new_with_config(path, Config {
            hashing_algorithm: Default::default(),
            encoding: Encoding::default(),
            id_generator: IdGenerator::default(),
            version: String::from(VERSION),
        })
    }

    /// Attempts creating a new repository with a specified config. Fails with `Error::AlreadyExists`
    /// if a repository already exists.
    pub fn new_with_config<P: Into<PathBuf>>(path: P, config: Config) -> Result<Self, Error> {
        let path: PathBuf = path.into();
        if path.is_dir() {
            Err(Error::AlreadyExists)
        } else {
            let mut config_path = path.clone();
            config_path.push(CONFIG_FILE);
            let mut issues_path = path.clone();
            issues_path.push(ISSUES_PATH);
            fs::create_dir_all(&issues_path)?;
            let repo = Repository {
                path,
                config_path,
                issues_path,
                config,
            };
            repo.save()?;
            Ok(repo)
        }

    }

    /// Opens an existing repository. Fails if there's no valid repository at the
    /// given path
    pub fn open<P: Into<PathBuf>>(path: P) -> Result<Self, Error> {
        let path: PathBuf = path.into();
        let mut config_path = path.clone();
        config_path.push(CONFIG_FILE);
        let mut issues_path = path.clone();
        issues_path.push(ISSUES_PATH);
        fs::create_dir_all(&issues_path)?;
        let file = fs::File::open(&config_path)?;
        let config: Config = serde_json::from_reader(file)?;
        if config.version != VERSION {
            return Err(Error::InvalidVersion { expected: String::from(VERSION), got: config.version });
        }
        let repository = Repository {
            path,
            config_path,
            issues_path,
            config,
        };
        Ok(repository)
    }

    pub fn find_in_or_above<P: Into<PathBuf>, S: AsRef<str>>(dir: S, path: P) -> Result<Self, Error> {
        let mut path: PathBuf = path.into();
        let dir = dir.as_ref();
        path.push(dir);
        loop {
            if !path.is_dir() {
                // get out of `dir`
                path.pop();
                // if can't pop anymore, we're at the root of the filesystem
                if !path.pop() {
                    return Err(Error::NotFound)
                }
                // try assuming current path + `dir`
                path.push(dir);
            } else {
                break;
            }
        }
        Repository::open(path)
    }


    /// Saves the repository. Ensures the directory exists and the configuration has
    /// been saved.
    fn save(&self) -> Result<(), Error> {
        fs::create_dir_all(&self.path)?;
        let file = fs::File::create(&self.config_path)?;
        serde_json::to_writer_pretty(file, &self.config)?;
        Ok(())
    }

    /// Returns repository path
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Returns issues path
    pub fn issues_path(&self) -> &Path {
        self.issues_path.as_path()
    }

    /// Returns repository's config
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns an unordered (as in "order not defined") issue iterator
    pub fn issue_iter(&self) -> Result<IssueIter, Error> {
        Ok(IssueIter { repository: self, dir: fs::read_dir(&self.issues_path)? })
    }

    /// Creates and returns a new issue with a unique ID
    pub fn new_issue(&self) -> Result<Issue, Error> {
        self.new_named_issue(self.config.id_generator.generate())
    }

    /// Creates and returns a new issue with a specific name. Will fail
    /// if there's an issue with the same name.
    pub fn new_named_issue<S: Into<String>>(&self, name: S) -> Result<Issue, Error> {
        let id: String = name.into();
        let mut path = self.issues_path.clone();
        path.push(&id);
        fs::create_dir(path)?;
        let id = OsString::from(id);
        Ok(Issue {
            repository: self,
            id,
        })
    }
}

use super::Issue as IssueTrait;

use std::ffi::{OsString, OsStr};

/// An issue residing in a repository
#[derive(Debug)]
pub struct Issue<'a> {
    repository: &'a Repository,
    id: OsString,
}

impl<'a> IssueTrait for Issue<'a> {

    type Error = Error;
    type Record = Record<'a>;
    type Records = Vec<Record<'a>>;
    type RecordIter = IssueRecordIter<'a>;

    fn id(&self) -> &str {
        self.id.to_str().unwrap()
    }

    fn record_iter(&self) -> Result<Self::RecordIter, Self::Error> {
        let path = self.repository.issues_path.join(PathBuf::from(&self.id()));
        let glob_pattern = format!("{}/**/*", path.to_str().unwrap());
        let dir = fs::read_dir(&path)?.filter(|r| r.is_ok())
            .map(|e| e.unwrap())
            .collect();
        let files: Vec<_> = glob::glob(&glob_pattern).expect("invalid glob pattern")
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap())
            .map(|f| f.strip_prefix(&path).unwrap().into())
            .collect();
        Ok(IssueRecordIter {
            issue: self.id.clone(),
            repository: self.repository,
            dir,
            files,
            parents: vec![],
        })
    }

    fn new_record<S: AsRef<str>, R: ::std::io::Read,
        I: Iterator<Item=(S, R)>>(&self, iter: I, link_parents: bool) -> Result<Self::Record, Self::Error> {
        use tempdir::TempDir;
        use std::io::{Read, Write};
        let tempdir = TempDir::new_in(&self.repository.path,"sit")?;
        let mut hasher = self.repository.config.hashing_algorithm.hasher();
        let mut buf = vec![0; 4096];

        fn process_file<S: AsRef<str>, R: ::std::io::Read>(hasher: &mut Hasher, name: S, mut reader: R, mut buf: &mut Vec<u8>, tempdir: &TempDir) -> Result<(), ::std::io::Error> {
            #[cfg(windows)] // replace backslashes with slashes
            let name_for_hashing = name.as_ref().replace("\\", "/");
            #[cfg(unix)]
            let name_for_hashing = name.as_ref();
            hasher.process((name_for_hashing.as_ref() as &str).as_bytes());
            let path = tempdir.path().join(PathBuf::from(name.as_ref() as &str));
            let mut dir = path.clone();
            dir.pop();
            fs::create_dir_all(dir)?;
            let mut file = fs::File::create(path)?;
            loop {
                let bytes_read = reader.read(&mut buf)?;
                hasher.process(&buf);
                file.write(&buf[0..bytes_read])?;
                if bytes_read == 0 {
                    break;
                }
            }
            Ok(())
        }

        let mut files: Vec<(Box<AsRef<str>>, Box<Read>)> = vec![];
        // iterate over all files
        for (name, mut reader) in iter {
            files.push((Box::new(name) as Box<AsRef<str>>, Box::new(reader) as Box<Read>));
        }

        // Link parents if requested
        if link_parents {
            match self.record_iter()?.last() {
                None => (),
                Some(records) => {
                    let parents = records.iter().map(|rec| (format!(".prev/{}", rec.encoded_hash()), &b""[..]));

                    for (name, mut reader) in parents {
                        files.push((Box::new(name) as Box<AsRef<str>>, Box::new(reader) as Box<Read>));
                    }
                },
            }
        }

        // IMPORTANT: Sort lexicographically
        files.sort_by(|&(ref name1, _), &(ref name2, _)|
            name1.as_ref().as_ref().cmp(name2.as_ref().as_ref()));


        for (name, mut reader) in files {
            process_file(&mut *hasher, name.as_ref(), reader, &mut buf, &tempdir)?;
        }

        let hash = hasher.result_box();
        fs::rename(tempdir.into_path(), self.repository.issues_path.join(PathBuf::from(self.id()))
            .join(PathBuf::from(self.repository.config.encoding.encode(&hash))))?;
        Ok(Record {
            hash,
            issue: self.id.clone(),
            repository: self.repository,
        })
    }
}

/// An iterator over records in an issue
pub struct IssueRecordIter<'a> {
    issue: OsString,
    repository: &'a Repository,
    dir: Vec<fs::DirEntry>,
    files: Vec<PathBuf>,
    parents: Vec<String>,
}

impl<'a> Iterator for IssueRecordIter<'a> {
    type Item = Vec<Record<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // top level
        if self.parents.len() == 0 {
            let result: Vec<_> = self.files.iter()
                // find issues
                .filter(|f|
                    f.components().count() == 1)
                // that don't have .prev/ID files in them
                .filter(|f| !self.files.iter()
                    .any(|f1| f1.starts_with(f) && f1.components().any(|c| c == Component::Normal(OsStr::new(".prev")))))
                // filter out invalid record names (if any)
                .filter(|f| self.repository.config.encoding.decode(f.to_str().unwrap().as_bytes()).is_ok())
                .map(|f| Record {
                    hash: self.repository.config.encoding.decode(f.to_str().unwrap().as_bytes()).unwrap(),
                    issue: self.issue.clone(),
                    repository: self.repository,
                })
                .collect();
            if result.len() == 0 {
                return None
            }
            self.parents = result.iter().map(|r| r.encoded_hash()).collect();
            return Some(result);
        } else {
            let result: Vec<_> = self.dir.iter()
                // filter out invalid record names (if any)
                .filter(|r| self.repository.config.encoding.decode(r.file_name().to_str().unwrap().as_bytes()).is_ok())
                .filter(|r| {
                    let links: Vec<_> = self.files.iter()
                        .filter(|f| f.starts_with(r.file_name()))
                        .filter(|f| {
                            let components: Vec<_> = f.components().skip(1).collect();
                            components.len() == 2 && components[0] == Component::Normal(OsStr::new(".prev"))
                        })
                        .map(|f| {
                            let components: Vec<_> = f.components().skip(2).collect();
                            PathBuf::from(components[0].as_os_str())
                        }).collect();
                    links.len() > 0 && links.iter().all(|l| self.parents.iter().any(|p| p == &String::from(l.to_str().unwrap())))
                })
                .map(|r| Record {
                    hash: self.repository.config.encoding.decode(r.file_name().to_str().unwrap().as_bytes()).unwrap(),
                    issue: self.issue.clone(),
                    repository: self.repository,
                })
                .collect();
            if result.len() == 0 {
                return None
            }
            self.parents = result.iter().map(|r| r.encoded_hash()).collect();
            return Some(result);
        }
    }
}


/// Unordered (as in "order not defined') issue iterator
/// within a repository
pub struct IssueIter<'a> {
    repository: &'a Repository,
    dir: fs::ReadDir,
}

impl<'a> Iterator for IssueIter<'a> {
    type Item = Issue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.dir.next() {
                None => return None,
                // bail on an entry if the entry is erroneous
                Some(Err(_)) => continue,
                Some(Ok(entry)) => {
                    let file_type = entry.file_type();
                    // bail on an entry if checking for the file type
                    // resulted in an error
                    if file_type.is_err() {
                        continue;
                    }
                    let file_type = file_type.unwrap();
                    if file_type.is_dir() {
                        return Some(Issue { repository: self.repository, id: entry.file_name() });
                    } else {
                        continue;
                    }
                }
            }
        }
    }
}

use super::Record as RecordTrait;

/// A record within an issue
#[derive(Debug)]
pub struct Record<'a> {
    hash: Vec<u8>,
    issue: OsString,
    repository: &'a Repository,
}

use serde::{Serialize, Serializer};

impl<'a> Serialize for Record<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        use record::RecordExt;
        self.serde_serialize(serializer)
    }
}


impl<'a> PartialEq for Record<'a> {
   fn eq(&self, other: &Record<'a>) -> bool {
       self.hash == other.hash
   }
}

impl<'a> RecordTrait for Record<'a> {
    type Read = ::std::fs::File;
    type Str = String;
    type Iter = RecordFileIterator<'a>;

    fn hash(&self) -> &[u8] {
        &self.hash
    }

    fn encoded_hash(&self) -> Self::Str {
        self.repository.config.encoding.encode(&self.hash)
    }

    fn file_iter(&self) -> Self::Iter {
        let path = self.repository.issues_path.join(PathBuf::from(&self.issue)).join(self.encoded_hash());
        let glob_pattern = format!("{}/**/*", path.to_str().unwrap());
        RecordFileIterator {
            glob: glob::glob(&glob_pattern).expect("invalid glob pattern"),
            repository: self.repository,
            issue: self.issue.clone(),
            record: self.encoded_hash(),
        }
    }

}

/// An iterator over files in a record
pub struct RecordFileIterator<'a> {
    glob: glob::Paths,
    repository: &'a Repository,
    issue: OsString,
    record: String,
}

impl<'a> Iterator for RecordFileIterator<'a> {
    type Item = (String, fs::File);

    fn next(&mut self) -> Option<Self::Item> {
        let prefix = self.repository.issues_path.join(PathBuf::from(&self.issue)).join(PathBuf::from(&self.record));
        loop {
            match self.glob.next() {
                None => return None,
                // skip on errors
                Some(Err(_)) => continue,
                Some(Ok(name)) => {
                    if name.is_file() {
                        let stripped = String::from(name.strip_prefix(&prefix).unwrap().to_str().unwrap());
                        #[cfg(windows)] // replace backslashes with slashes
                        let stripped = stripped.replace("\\", "/");
                        return Some((stripped, fs::File::open(name).unwrap()))
                    } else {
                        // if it is not a file, keep iterating
                        continue
                    }
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {

    use tempdir::TempDir;

    use super::*;

    #[test]
    fn new_repo() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        assert_eq!(repo.issue_iter().unwrap().count(), 0); // no issues in a new repo
        assert_eq!(repo.path(), tmp);
    }

    #[test]
    fn new_repo_already_exists() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let _repo = Repository::new(&tmp).unwrap();
        // try creating it again
        let repo = Repository::new(&tmp);
        assert!(repo.is_err());
        assert_matches!(repo.unwrap_err(), Error::AlreadyExists);
    }

    #[test]
    fn open_repo() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an issue
        let issue = repo.new_issue().unwrap();
        let repo = Repository::open(&tmp).unwrap();
        // load issues
        let mut issues: Vec<Issue> = repo.issue_iter().unwrap().collect();
        assert_eq!(issues.len(), 1);
        // check equality of the issue's ID
        assert_eq!(issues.pop().unwrap().id(), issue.id());
    }

    #[test]
    fn find_repo() {
        let tmp = TempDir::new("sit").unwrap().into_path();
        let sit = tmp.join(".sit");
        // create repo
        Repository::new(&sit).unwrap();
        let deep_subdir = tmp.join("a/b/c/d");
        let repo = Repository::find_in_or_above(".sit", &deep_subdir).unwrap();
        assert_eq!(repo.path(), sit);
        // negative test
        assert_matches!(Repository::find_in_or_above(".sit-dir", &deep_subdir), Err(Error::NotFound));
    }

    #[test]
    fn new_issue() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an issue
        let issue = repo.new_issue().unwrap();
        // load issues
        let mut issues: Vec<Issue> = repo.issue_iter().unwrap().collect();
        assert_eq!(issues.len(), 1);
        // check equality of the issue's ID
        assert_eq!(issues.pop().unwrap().id(), issue.id());
    }

    #[test]
    fn new_named_issue() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an issue
        let issue = repo.new_named_issue("one").unwrap();
        // load issues
        let mut issues: Vec<Issue> = repo.issue_iter().unwrap().collect();
        assert_eq!(issues.len(), 1);
        // check equality of the issue's ID
        assert_eq!(issues.pop().unwrap().id(), issue.id());
    }

    #[test]
    fn new_named_issue_dup() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an issue
        let _issue = repo.new_named_issue("one").unwrap();
        // attempt to use the same name
        let issue1 = repo.new_named_issue("one");
        assert!(issue1.is_err());
        assert_matches!(issue1.unwrap_err(), Error::IoError(_));
        // there's still just one issue
        assert_eq!(repo.issue_iter().unwrap().count(), 1);
    }

    #[test]
    fn new_record() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an issue
        let issue = repo.new_issue().unwrap();
        // create a record
        let record = issue.new_record(vec![("test", &b"hello"[..])].into_iter(), true).unwrap();
        // peek into the record
        let mut files: Vec<_> = record.file_iter().collect();
        assert_eq!(files.len(), 1);
        let (name, mut file) = files.pop().unwrap();
        assert_eq!(name, "test");
        use std::io::Read;
        let mut string = String::new();
        assert!(file.read_to_string(&mut string).is_ok());
        assert_eq!(string, "hello");
        // list records
        let mut records: Vec<Record> = issue.record_iter().unwrap().flat_map(|v| v).collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records.pop().unwrap().hash(), record.hash());
    }


    #[test]
    fn new_record_parents_linking() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an issue
        let issue = repo.new_issue().unwrap();
        // create a few top records
        let record1 = issue.new_record(vec![("test", &[1u8][..])].into_iter(), false).unwrap();
        let record1link = format!(".prev/{}", record1.encoded_hash());
        let record2 = issue.new_record(vec![("test", &[2u8][..])].into_iter(), false).unwrap();
        let record2link = format!(".prev/{}", record2.encoded_hash());
        // now attempt to create a record that should link both together
        let record = issue.new_record(vec![("test", &[3u8][..])].into_iter(), true).unwrap();
        assert!(record.file_iter().any(|(name, _)| name == *&record1link));
        assert!(record.file_iter().any(|(name, _)| name == *&record2link));
    }

    #[test]
    fn record_ordering() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an issue
        let issue = repo.new_issue().unwrap();
        // create a few top records
        let record1 = issue.new_record(vec![("test", &[1u8][..])].into_iter(), false).unwrap();
        let record2 = issue.new_record(vec![("test", &[2u8][..])].into_iter(), false).unwrap();
        // now attempt to create a record that should link both together
        let record3 = issue.new_record(vec![("test", &[3u8][..])].into_iter(), true).unwrap();
        // and another top record
        let record4 = issue.new_record(vec![("test", &[4u8][..])].into_iter(), false).unwrap();
        // and another linking record
        let record5 = issue.new_record(vec![("test", &[5u8][..])].into_iter(), true).unwrap();

        // now, look at their ordering
        let mut records: Vec<_> = issue.record_iter().unwrap().collect();
        let row_3 = records.pop().unwrap();
        let row_2 = records.pop().unwrap();
        let row_1 = records.pop().unwrap();
        assert_eq!(records.len(), 0);

        assert_eq!(row_1.len(), 3);
        assert!(row_1.iter().any(|r| r == &record1));
        assert!(row_1.iter().any(|r| r == &record2));
        assert!(row_1.iter().any(|r| r == &record4));

        assert_eq!(row_2.len(), 1);
        assert!(row_2.iter().any(|r| r == &record3));

        assert_eq!(row_3.len(), 1);
        assert!(row_3.iter().any(|r| r == &record5));
    }

    #[test]
    fn record_deterministic_hashing() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        let issue1 = repo.new_issue().unwrap();
        let record1 = issue1.new_record(vec![("z/a", &[2u8][..]), ("test", &[1u8][..])].into_iter(), false).unwrap();
        let issue2 = repo.new_issue().unwrap();
        let record2 = issue2.new_record(vec![("test", &[1u8][..]), ("z/a", &[2u8][..])].into_iter(), false).unwrap();
        assert_eq!(record1.hash(), record2.hash());
        #[cfg(windows)] {
            let issue3 = repo.new_issue().unwrap();
            let record3 = issue3.new_record(vec![("test", &[1u8][..]), ("z\\a", &[2u8][..])].into_iter(), false).unwrap();
            assert_eq!(record3.hash(), record2.hash());
        }
    }


}
