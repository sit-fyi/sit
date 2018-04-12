//! Repository is where SIT stores all of its artifacts.
//!
//! It is represented by the [`Repository`] structure.
//!
//! [`Repository`]: struct.Repository.html
//!


use std::path::{Path, PathBuf};
use std::fs;
use std::io::{Read, Write};

use tempdir::TempDir;

use glob;

use serde_json;

use super::hash::{HashingAlgorithm, Hasher};
use super::encoding::Encoding;
use super::id::IdGenerator;

use std::collections::HashMap;

use std::marker::PhantomData;

/// Current repository format version
const VERSION: &str = "1";
/// Repository's config file name
const CONFIG_FILE: &str = "config.json";
/// Repository's issues path (deprecated)
const DEPRECATED_ISSUES_PATH: &str = "issues";
/// Repository's items path
const ITEMS_PATH: &str = "items";
/// Repository's modules path
const MODULES_PATH: &str = "modules";


/// Repository is the container for all SIT artifacts
#[derive(Debug)]
pub struct Repository {
    /// Path to the container
    path: PathBuf,
    /// Path to the config file. Mainly to avoid creating
    /// this path on demand for every operation that would
    /// require it
    config_path: PathBuf,
    /// Path to the modules. Mainly to avoid creating
    /// this path on demand for every operation that would
    /// require it
    modules_path: PathBuf,
    /// Path to items. Mainly to avoid creating this path
    /// on demand for every operation that would require it
    items_path: PathBuf,
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
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(PartialEq, Debug)]
pub enum Upgrade {
    IssuesToItems,
}

use std::fmt::{self, Display};

impl Display for Upgrade {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &Upgrade::IssuesToItems => write!(f, "renaming issues/ to items/"),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    /// Item already exists
    AlreadyExists,
    /// Item not found
    NotFound,
    /// Upgrade required
    #[error(no_from, non_std)]
    UpgradeRequired(Upgrade),
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

#[allow(unused_variables,dead_code)]
mod default_files {
    include!(concat!(env!("OUT_DIR"), "/default_files.rs"));

    use std::path::PathBuf;
    use std::collections::HashMap;

    lazy_static! {
      pub static ref ASSETS: HashMap<PathBuf, File> = {
         let mut map = HashMap::new();
         let prefix = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("default-files");
         for entry in FILES.walk() {
            match entry {
               DirEntry::File(f) => {
                  let path = PathBuf::from(f.path().strip_prefix(&prefix).unwrap());
                  map.insert(path.clone(), f.clone());
               },
               _ => (),
            }
         }
         map
       };
    }

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
            extra: HashMap::new(),
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
            let mut items_path = path.clone();
            items_path.push(ITEMS_PATH);
            fs::create_dir_all(&items_path)?;
            let modules_path = path.join(MODULES_PATH);
            let repo = Repository {
                path,
                config_path,
                items_path,
                config,
                modules_path,
            };
            repo.save()?;
            Ok(repo)
        }

    }

    /// Opens an existing repository. Fails if there's no valid repository at the
    /// given path
    pub fn open<P: Into<PathBuf>>(path: P) -> Result<Self, Error> {
        Repository::open_and_upgrade(path, &[])
    }

    /// Opens and, if necessary, upgrades an existing repository.
    /// Allow to specify which particular upgrades should be allowed.
    ///
    /// Fails if there's no valid repository at the
    /// given path.
    pub fn open_and_upgrade<P: Into<PathBuf>, U: AsRef<[Upgrade]>>(path: P, upgrades: U) -> Result<Self, Error> {
        let path: PathBuf = path.into();
        let mut config_path = path.clone();
        config_path.push(CONFIG_FILE);
        let issues_path = path.join(DEPRECATED_ISSUES_PATH);
        let items_path = path.join(ITEMS_PATH);
        let modules_path = path.join(MODULES_PATH);
        if issues_path.is_dir() && !items_path.is_dir() {
            if upgrades.as_ref().contains(&Upgrade::IssuesToItems) {
                fs::rename(&issues_path, &items_path)?;
            } else {
                return Err(Error::UpgradeRequired(Upgrade::IssuesToItems));
            }
        }
        if issues_path.is_dir() && items_path.is_dir() {
            if upgrades.as_ref().contains(&Upgrade::IssuesToItems) {
                for item in fs::read_dir(&issues_path)?.filter(Result::is_ok).map(Result::unwrap) {
                    fs::rename(item.path(), items_path.join(item.file_name()))?;
                }
                fs::remove_dir_all(&issues_path)?;
            } else {
                return Err(Error::UpgradeRequired(Upgrade::IssuesToItems));
            }
        }
        // dropping issues_path so it can no longer be used
        // by mistake
        drop(issues_path);
        fs::create_dir_all(&items_path)?;
        let file = fs::File::open(&config_path)?;
        let config: Config = serde_json::from_reader(file)?;
        if config.version != VERSION {
            return Err(Error::InvalidVersion { expected: String::from(VERSION), got: config.version });
        }
        let repository = Repository {
            path,
            config_path,
            items_path,
            config,
            modules_path,
        };
        Ok(repository)
    }

    /// Given relative path of `path` (such as ".sit"), finds a repository in a directory or above
    pub fn find_in_or_above<P: Into<PathBuf>, S: AsRef<str>>(dir: S, path: P) -> Option<PathBuf> {
        let mut path: PathBuf = path.into();
        let dir = dir.as_ref();
        path.push(dir);
        loop {
            if !path.is_dir() {
                // get out of `dir`
                path.pop();
                // if can't pop anymore, we're at the root of the filesystem
                if !path.pop() {
                    return None
                }
                // try assuming current path + `dir`
                path.push(dir);
            } else {
                break;
            }
        }
        Some(path)
    }


    /// Saves the repository. Ensures the directory exists and the configuration has
    /// been saved.
    fn save(&self) -> Result<(), Error> {
        fs::create_dir_all(&self.path)?;
        let file = fs::File::create(&self.config_path)?;
        serde_json::to_writer_pretty(file, &self.config)?;
        Ok(())
    }

    /// Populates repository with default files
    pub fn populate_default_files(&self) -> Result<(), Error> {
        for (name, file) in default_files::ASSETS.iter() {
            let mut dir = self.path.join(name);
            dir.pop();
            fs::create_dir_all(dir)?;
            let mut f = fs::File::create(self.path.join(name))?;
            f.write(file.contents)?;
        }
        Ok(())
    }

    /// Returns repository path
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Returns items path
    pub fn items_path(&self) -> &Path {
        self.items_path.as_path()
    }

    /// Returns repository's config
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns an unordered (as in "order not defined") item iterator
    pub fn item_iter(&self) -> Result<ItemIter, Error> {
        Ok(ItemIter { repository: self, dir: fs::read_dir(&self.items_path)? })
    }

    /// Creates and returns a new item with a unique ID
    pub fn new_item(&self) -> Result<Item, Error> {
        self.new_named_item(self.config.id_generator.generate())
    }

    /// Creates and returns a new item with a specific name. Will fail
    /// if there's an item with the same name.
    pub fn new_named_item<S: Into<String>>(&self, name: S) -> Result<Item, Error> {
        let id: String = name.into();
        let mut path = self.items_path.clone();
        path.push(&id);
        fs::create_dir(path)?;
        let id = OsString::from(id);
        Ok(Item {
            repository: self,
            id,
        })
    }

    /// Returns path to modules. The target directory may not exist.
    pub fn modules_path(&self) -> &Path {
        &self.modules_path
    }

    /// Returns an iterator over the list of modules (directories under `modules` directory)
    pub fn module_iter(&self) -> Result<Box<Iterator<Item = PathBuf>>, Error> {
        let path = self.path.join("modules");
        if !path.is_dir() {
            return Ok(Box::new(vec![].into_iter()));
        }
        let modules = fs::read_dir(path)?;

        Ok(Box::new(modules.filter(Result::is_ok).map(Result::unwrap)
            .map(|f| {
                let mut path = f.path();
                if path.is_dir() {
                    return path
                } else {
                    let mut f = fs::File::open(&path).unwrap();
                    use std::io::Read;
                    let mut s = String::new();
                    f.read_to_string(&mut s).unwrap();
                    #[cfg(windows)] {
                        s = s.replace("/", "\\");
                    }
                    let trimmed_path = s.trim();
                    path.pop(); // remove the file name
                    path.join(PathBuf::from(trimmed_path))
               }
            })))
    }
}

use super::Item as ItemTrait;

use std::ffi::OsString;

/// An item residing in a repository
#[derive(Debug)]
pub struct Item<'a> {
    repository: &'a Repository,
    id: OsString,
}


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
impl<'a> Item<'a> {
    pub fn new_record_in<P: AsRef<Path>, S: AsRef<str>, R: ::std::io::Read,
        I: Iterator<Item=(S, R)>>(&self, path: P, iter: I, link_parents: bool) -> Result<<Item<'a> as ItemTrait>::Record, <Item<'a> as ItemTrait>::Error> {
        let tempdir = TempDir::new_in(&self.repository.path,"sit")?;
        let mut hasher = self.repository.config.hashing_algorithm.hasher();
        let mut buf = vec![0; 4096];

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
        let actual_path = path.as_ref().join(PathBuf::from(self.repository.config.encoding.encode(&hash)));
        fs::rename(tempdir.into_path(), &actual_path)?;
        Ok(Record {
            hash,
            item: self.id.clone(),
            repository: self.repository,
            actual_path,
        })
    }

}
impl<'a> ItemTrait for Item<'a> {

    type Error = Error;
    type Record = Record<'a>;
    type Records = Vec<Record<'a>>;
    type RecordIter = ItemRecordIter<'a>;

    fn id(&self) -> &str {
        self.id.to_str().unwrap()
    }

    fn record_iter(&self) -> Result<Self::RecordIter, Self::Error> {
        let path = self.repository.items_path.join(PathBuf::from(&self.id()));
        let dir = fs::read_dir(&path)?.filter(|r| r.is_ok())
            .map(|e| e.unwrap())
            .collect();
        Ok(ItemRecordIter {
            item: self.id.clone(),
            repository: self.repository,
            dir,
            parents: vec![],
        })
    }

    fn new_record<S: AsRef<str>, R: ::std::io::Read,
        I: Iterator<Item=(S, R)>>(&self, iter: I, link_parents: bool) -> Result<Self::Record, Self::Error> {
       self.new_record_in(self.repository.items_path.join(PathBuf::from(self.id())), iter, link_parents)
    }

}

/// An iterator over records in an item
pub struct ItemRecordIter<'a> {
    item: OsString,
    repository: &'a Repository,
    dir: Vec<fs::DirEntry>,
    parents: Vec<String>,
}

impl<'a> Iterator for ItemRecordIter<'a> {
    type Item = Vec<Record<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // top level
        if self.parents.len() == 0 {
            let result: Vec<_> = self.dir.iter()
                .filter(|e| e.file_type().unwrap().is_dir())
                // find items
                // that don't have .prev/ID files in them
                .filter(|e|
                            !fs::read_dir(e.path()).unwrap()
                                .filter(Result::is_ok)
                                .map(Result::unwrap)
                                .any(|e| e.file_name().to_str().unwrap() == ".prev")
                )
                .map(|e| e.file_name())
                // filter out invalid record names (if any)
                .filter(|f| self.repository.config.encoding.decode(f.to_str().unwrap().as_bytes()).is_ok())
                .map(|f| Record {
                    hash: self.repository.config.encoding.decode(f.to_str().unwrap().as_bytes()).unwrap(),
                    item: self.item.clone(),
                    repository: self.repository,
                    actual_path: self.repository.items_path().join(&self.item).join(f.to_str().unwrap()),
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
                .filter(|e| self.repository.config.encoding.decode(e.file_name().to_str().unwrap().as_bytes()).is_ok())
                .filter(|e| {
                    let links: Vec<_> = match fs::read_dir(e.path().join(".prev")) {
                        Err(_) => vec![],
                        Ok(dir) => dir
                            .filter(Result::is_ok)
                            .map(Result::unwrap)
                            .map(|e| String::from(e.file_name().to_str().unwrap()))
                            .collect(),
                    };
                    links.len() > 0 && links.iter().all(|l| self.parents.iter().any(|p| p == l))
                })
                .map(|e| Record {
                    hash: self.repository.config.encoding.decode(e.file_name().to_str().unwrap().as_bytes()).unwrap(),
                    item: self.item.clone(),
                    repository: self.repository,
                    actual_path: self.repository.items_path().join(&self.item).join(e.file_name()),
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


/// Unordered (as in "order not defined') item iterator
/// within a repository
pub struct ItemIter<'a> {
    repository: &'a Repository,
    dir: fs::ReadDir,
}

impl<'a> Iterator for ItemIter<'a> {
    type Item = Item<'a>;

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
                        return Some(Item { repository: self.repository, id: entry.file_name() });
                    } else {
                        continue;
                    }
                }
            }
        }
    }
}

use super::Record as RecordTrait;

/// A record within an item
#[derive(Debug)]
pub struct Record<'a> {
    hash: Vec<u8>,
    item: OsString,
    repository: &'a Repository,
    actual_path: PathBuf,
}

/// Somethiing that can provide access to its underlying repository
pub trait RepositoryProvider {
    /// Returns underlying repository;
    fn repository(&self) -> &Repository;
}

impl<'a> RepositoryProvider for Record<'a> {
    fn repository(&self) -> &Repository {
        self.repository
    }
}

#[derive(Debug)]
/// Record wrapper that dynamically rehashes wrapped Record's content
pub struct DynamicallyHashedRecord<'a, T: RecordTrait + RepositoryProvider + 'a>(&'a T);

impl<'a, T: RecordTrait<Str=String, Hash=Vec<u8>> + RepositoryProvider + 'a> RecordTrait for DynamicallyHashedRecord<'a, T> {
    type Read = T::Read;
    type Str = String;
    type Hash = Vec<u8>;
    type Iter = T::Iter;

    fn hash(&self) -> Self::Hash {
        let tempdir = TempDir::new_in(&self.0.repository().path(),"sit").unwrap();
        let mut hasher = self.0.repository().config.hashing_algorithm.hasher();
        let mut buf = vec![0; 4096];

        let mut files: Vec<(Box<AsRef<str>>, Box<Read>)> = vec![];

        for (name, reader) in self.file_iter() {
            files.push((Box::new(name) as Box<AsRef<str>>, Box::new(reader) as Box<Read>));
        }

        // IMPORTANT: Sort lexicographically
        files.sort_by(|&(ref name1, _), &(ref name2, _)|
            name1.as_ref().as_ref().cmp(name2.as_ref().as_ref()));

        for (name, mut reader) in files {
            process_file(&mut *hasher, name.as_ref(), reader, &mut buf, &tempdir).unwrap();
        }

        hasher.result_box()
    }

    fn encoded_hash(&self) -> Self::Str {
        self.0.repository().config.encoding.encode(self.hash().as_ref())
    }

    fn file_iter(&self) -> Self::Iter {
        self.0.file_iter()
    }

    fn item_id(&self) -> Self::Str {
        self.0.item_id()
    }
}

#[derive(Debug)]
/// Record with filtered content
pub struct FilteredRecord<'a, S: AsRef<str>, R: Read, T: RecordTrait<Str=S, Read=R> + RepositoryProvider + 'a,
           F: Fn(&(S, R)) -> bool>(&'a T, F);

impl<'a, S: AsRef<str>, R: Read, T: RecordTrait<Str=S, Read=R> + RepositoryProvider + 'a, F: Copy + Fn(&(S, R)) -> bool> RecordTrait for FilteredRecord<'a, S, R, T, F> {
    type Read = T::Read;
    type Hash = T::Hash;
    type Str = T::Str;
    type Iter = ::std::iter::Filter<T::Iter, F>;

    fn hash(&self) -> Self::Hash {
        self.0.hash()
    }

    fn encoded_hash(&self) -> Self::Str {
        self.0.encoded_hash()
    }

    fn file_iter(&self) -> Self::Iter {
        self.0.file_iter().filter(self.1)
    }

    fn item_id(&self) -> Self::Str {
        self.0.item_id()
    }
}

impl <'a, S: AsRef<str>, R: Read, T: RecordTrait<Str=S, Read=R> + RepositoryProvider + 'a, F: Copy + Fn(&(S, R)) -> bool> RepositoryProvider for FilteredRecord<'a, S, R, T, F> {
    fn repository(&self) -> &Repository {
        self.0.repository()
    }
}

/// Allows any Record to have its content dynamically rehashed
pub trait DynamicallyHashable<'a> : RecordTrait + RepositoryProvider + Sized {
    /// Returns a record that has its hash dynamically computed
    fn dynamically_hashed(&'a self) -> DynamicallyHashedRecord<'a, Self> {
        DynamicallyHashedRecord(self)
    }
}

impl<'a> DynamicallyHashable<'a> for Record<'a> {}
impl<'a, S: AsRef<str>, R: Read, T: RecordTrait<Str=S, Read=R> + RepositoryProvider + 'a, F: Copy + Fn(&(S, R)) -> bool> DynamicallyHashable<'a> for FilteredRecord<'a, S, R, T, F> {}

impl<'a> Record<'a> {

    /// Returns path to the record, as it should be per repository's naming scheme
    ///
    /// The record MIGHT not be at this path as this is the path where
    /// it SHOULD BE. The actual path can be retrieved using `actual_path()`
    pub fn path(&self) -> PathBuf {
        self.repository.items_path.join(PathBuf::from(&self.item)).join(self.encoded_hash())
    }

    /// Returns an actual path to the record directory
    pub fn actual_path(&self) -> &Path {
        self.actual_path.as_path()
    }


    /// Returns a record with filtered files
    pub fn filtered<F>(&'a self, filter: F) -> FilteredRecord<'a, <Record<'a> as RecordTrait>::Str,
        <Record<'a> as RecordTrait>::Read,
        Record<'a>, F> where F: Fn(&(<Record<'a> as RecordTrait>::Str, <Record<'a> as RecordTrait>::Read)) -> bool {
        FilteredRecord(self, filter)
    }

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
    type Hash = Vec<u8>;
    type Iter = RecordFileIterator<'a>;

    fn hash(&self) -> Self::Hash {
        self.hash.clone()
    }

    fn encoded_hash(&self) -> Self::Str {
        self.repository.config.encoding.encode(&self.hash)
    }

    fn file_iter(&self) -> Self::Iter {
        let path = self.actual_path();
        let glob_pattern = format!("{}/**/*", path.to_str().unwrap());
        RecordFileIterator {
            glob: glob::glob(&glob_pattern).expect("invalid glob pattern"),
            prefix: self.actual_path().into(),
            phantom: PhantomData,
        }
    }
    fn item_id(&self) -> Self::Str {
        self.item.clone().into_string().unwrap()
    }
}

/// An iterator over files in a record
pub struct RecordFileIterator<'a> {
    glob: glob::Paths,
    prefix: PathBuf,
    phantom: PhantomData<&'a ()>,
}

impl<'a> Iterator for RecordFileIterator<'a> {
    type Item = (String, fs::File);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.glob.next() {
                None => return None,
                // skip on errors
                Some(Err(_)) => continue,
                Some(Ok(name)) => {
                    if name.is_file() {
                        let stripped = String::from(name.strip_prefix(&self.prefix).unwrap().to_str().unwrap());
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
        assert_eq!(repo.item_iter().unwrap().count(), 0); // no items in a new repo
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
        // create an item
        let item = repo.new_item().unwrap();
        let repo = Repository::open(&tmp).unwrap();
        // load items
        let mut items: Vec<Item> = repo.item_iter().unwrap().collect();
        assert_eq!(items.len(), 1);
        // check equality of the item's ID
        assert_eq!(items.pop().unwrap().id(), item.id());
    }

    #[test]
    fn find_repo() {
        let tmp = TempDir::new("sit").unwrap().into_path();
        let sit = tmp.join(".sit");
        // create repo
        Repository::new(&sit).unwrap();
        let deep_subdir = tmp.join("a/b/c/d");
        let repo = Repository::find_in_or_above(".sit", &deep_subdir);
        assert!(repo.is_some());
        let repo = Repository::open(repo.unwrap()).unwrap();
        assert_eq!(repo.path(), sit);
        // negative test
        assert!(Repository::find_in_or_above(".sit-dir", &deep_subdir).is_none());
    }

    #[test]
    fn new_item() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let item = repo.new_item().unwrap();
        // load items
        let mut items: Vec<Item> = repo.item_iter().unwrap().collect();
        assert_eq!(items.len(), 1);
        // check equality of the item's ID
        assert_eq!(items.pop().unwrap().id(), item.id());
    }

    #[test]
    fn new_named_item() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let item = repo.new_named_item("one").unwrap();
        // load items
        let mut items: Vec<Item> = repo.item_iter().unwrap().collect();
        assert_eq!(items.len(), 1);
        // check equality of the item's ID
        assert_eq!(items.pop().unwrap().id(), item.id());
    }

    #[test]
    fn new_named_item_dup() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let _item = repo.new_named_item("one").unwrap();
        // attempt to use the same name
        let item1 = repo.new_named_item("one");
        assert!(item1.is_err());
        assert_matches!(item1.unwrap_err(), Error::IoError(_));
        // there's still just one item
        assert_eq!(repo.item_iter().unwrap().count(), 1);
    }

    #[test]
    fn new_record() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let item = repo.new_item().unwrap();
        // create a record
        let record = item.new_record(vec![("test", &b"hello"[..])].into_iter(), true).unwrap();
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
        let mut records: Vec<Record> = item.record_iter().unwrap().flat_map(|v| v).collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records.pop().unwrap().hash(), record.hash());
    }


    #[test]
    fn new_record_parents_linking() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let item = repo.new_item().unwrap();
        // create a few top records
        let record1 = item.new_record(vec![("test", &[1u8][..])].into_iter(), false).unwrap();
        let record1link = format!(".prev/{}", record1.encoded_hash());
        let record2 = item.new_record(vec![("test", &[2u8][..])].into_iter(), false).unwrap();
        let record2link = format!(".prev/{}", record2.encoded_hash());
        // now attempt to create a record that should link both together
        let record = item.new_record(vec![("test", &[3u8][..])].into_iter(), true).unwrap();
        assert!(record.file_iter().any(|(name, _)| name == *&record1link));
        assert!(record.file_iter().any(|(name, _)| name == *&record2link));
    }

    #[test]
    fn record_ordering() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let item = repo.new_item().unwrap();
        // create a few top records
        let record1 = item.new_record(vec![("test", &[1u8][..])].into_iter(), false).unwrap();
        let record2 = item.new_record(vec![("test", &[2u8][..])].into_iter(), false).unwrap();
        // now attempt to create a record that should link both together
        let record3 = item.new_record(vec![("test", &[3u8][..])].into_iter(), true).unwrap();
        // and another top record
        let record4 = item.new_record(vec![("test", &[4u8][..])].into_iter(), false).unwrap();
        // and another linking record
        let record5 = item.new_record(vec![("test", &[5u8][..])].into_iter(), true).unwrap();

        // now, look at their ordering
        let mut records: Vec<_> = item.record_iter().unwrap().collect();
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
        let item1 = repo.new_item().unwrap();
        let record1 = item1.new_record(vec![("z/a", &[2u8][..]), ("test", &[1u8][..])].into_iter(), false).unwrap();
        let item2 = repo.new_item().unwrap();
        let record2 = item2.new_record(vec![("test", &[1u8][..]), ("z/a", &[2u8][..])].into_iter(), false).unwrap();
        assert_eq!(record1.hash(), record2.hash());
        #[cfg(windows)] {
            let item3 = repo.new_item().unwrap();
            let record3 = item3.new_record(vec![("test", &[1u8][..]), ("z\\a", &[2u8][..])].into_iter(), false).unwrap();
            assert_eq!(record3.hash(), record2.hash());
        }
    }

    #[test]
    fn record_dynamic_hashing() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        let item = repo.new_item().unwrap();
        let record = item.new_record(vec![("z/a", &[2u8][..]), ("test", &[1u8][..])].into_iter(), false).unwrap();
        let record_dynamic = record.dynamically_hashed();
        assert_eq!(record_dynamic.hash(), record.hash());
        assert_eq!(record_dynamic.encoded_hash(), record.encoded_hash());
        // now, put some file in the dynamic one
        let hash = record.hash();
        let encoded_hash = record.encoded_hash();
        ::std::fs::File::create(record.path().join("dynamic")).unwrap();
        assert_eq!(record.hash(), hash);
        assert_eq!(record.encoded_hash(), encoded_hash);
        assert_ne!(record_dynamic.hash(), record.hash());
        assert_ne!(record_dynamic.encoded_hash(), record.encoded_hash());
    }

    #[test]
    fn record_filtering() {
         let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        let item = repo.new_item().unwrap();
        let record = item.new_record(vec![("z/a", &[2u8][..]), ("test", &[1u8][..])].into_iter(), false).unwrap();
        fn not_za(val: &(String, fs::File)) -> bool {
            val.0 != "z/a"
        }
        let filtered = record.filtered(not_za);
        // Check the content
        assert_eq!(filtered.file_iter().count(), 1);
        let files: Vec<_> = filtered.file_iter().map(|(name, _)| name).collect();
        assert_eq!(files, vec!["test"]);
        // Filtering alone doesn't change hash
        assert_eq!(filtered.hash(), record.hash());
        assert_eq!(filtered.encoded_hash(), record.encoded_hash());
        // But doing it dynamically does
        assert_ne!(filtered.dynamically_hashed().hash(), record.hash());
        assert_ne!(filtered.dynamically_hashed().encoded_hash(), record.encoded_hash());
    }

    #[test]
    fn record_outside_naming_scheme() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let mut tmp1 = tmp.clone();
        tmp1.pop();

        let repo = Repository::new(&tmp).unwrap();
        let item = repo.new_item().unwrap();
        let _record1 = item.new_record(vec![("z/a", &[2u8][..]), ("test", &[1u8][..])].into_iter(), false).unwrap();
        let record2 = item.new_record_in(&tmp1, vec![("a", &[2u8][..])].into_iter(), true).unwrap();

        // lets test that record2 can iterate over correct files
        let files: Vec<_> = record2.file_iter().collect();
        assert_eq!(files.len(), 2); // a and .prev/...


        // record2 can't be found as it is outside of the standard naming scheme
        let records: Vec<Vec<_>> = item.record_iter().unwrap().collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].len(), 1);

        // On Windows, if a file within a directory that is being
        // moved is open (even for reading), this will prevent
        // this said directory from being moved, returning "Access Denied"
        // Therefore, we drop `files` here to release the `File` readers
        #[cfg(windows)]
        drop(files);

        ::std::fs::rename(record2.actual_path(), record2.path()).unwrap();

        // and now it can be
        let records: Vec<Vec<_>> = item.record_iter().unwrap().collect();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].len(), 1);
        assert_eq!(records[0].len(), 1);

    }

    #[test]
    fn issues_to_items_upgrade() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let mut tmp1 = tmp.clone();
        tmp1.pop();

        let repo = Repository::new(&tmp).unwrap();
        let _item = repo.new_item().unwrap();
        assert_eq!(repo.item_iter().unwrap().count(), 1);

        ::std::fs::rename(repo.items_path(), repo.path().join("issues")).unwrap();
        let repo = Repository::open(&tmp);
        assert!(repo.is_err());
        assert_matches!(repo.unwrap_err(), Error::UpgradeRequired(Upgrade::IssuesToItems));

        let repo = Repository::open_and_upgrade(&tmp, &[Upgrade::IssuesToItems]).unwrap();
        assert!(!repo.path().join("issues").exists());
        assert_eq!(repo.item_iter().unwrap().count(), 1);

        // now, a more complicated case:
        // both issues/ and items/ are present
        // this can happen when merging a patch that changes .sit/issues
        // (prepared before the migration)
        let item = repo.new_item().unwrap();
        ::std::fs::create_dir_all(repo.path().join("issues")).unwrap();
        ::std::fs::rename(repo.items_path().join(item.id()), repo.path().join("issues").join(item.id())).unwrap();

        let repo = Repository::open(&tmp);
        assert!(repo.is_err());
        assert_matches!(repo.unwrap_err(), Error::UpgradeRequired(Upgrade::IssuesToItems));

        let repo = Repository::open_and_upgrade(&tmp, &[Upgrade::IssuesToItems]).unwrap();
        assert!(!repo.path().join("issues").exists());
        assert_eq!(repo.item_iter().unwrap().count(), 2);

    }

    #[test]
    fn modules() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let mut tmp1 = tmp.clone();
        tmp1.pop();

        let repo = Repository::new(&tmp).unwrap();
        assert!(repo.module_iter().unwrap().next().is_none());

        // create modules/test
        let path = repo.modules_path().join("test");
        fs::create_dir_all(&path).unwrap();
        let mut iter = repo.module_iter().unwrap();

        assert_eq!(::dunce::canonicalize(iter.next().unwrap()).unwrap(), ::dunce::canonicalize(path).unwrap());
        assert!(iter.next().is_none());
    }

    #[test]
    fn link_module_absolute() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let mut tmp1 = tmp.clone();
        tmp1.pop();

        let tmp2 = TempDir::new("sit-mod").unwrap().into_path();

        let repo = Repository::new(&tmp).unwrap();
        assert!(repo.module_iter().unwrap().next().is_none());

        // create modules/test
        fs::create_dir_all(repo.modules_path()).unwrap();
        let mut f = fs::File::create(repo.modules_path().join("test")).unwrap();
        f.write(tmp2.to_str().unwrap().as_bytes()).unwrap();

        let mut iter = repo.module_iter().unwrap();

        assert_eq!(iter.next().unwrap(), tmp2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn link_module_relative() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let mut tmp1 = tmp.clone();
        tmp1.pop();



        let repo = Repository::new(&tmp).unwrap();
        fs::create_dir_all(tmp.join("module1")).unwrap();

        assert!(repo.module_iter().unwrap().next().is_none());

        // create modules/test
        fs::create_dir_all(repo.modules_path()).unwrap();
        let mut f = fs::File::create(repo.modules_path().join("test")).unwrap();
        f.write(b"../module1").unwrap();

        let mut iter = repo.module_iter().unwrap();

        assert_eq!(::dunce::canonicalize(iter.next().unwrap()).unwrap(), ::dunce::canonicalize(tmp.join("module1")).unwrap());
        assert!(iter.next().is_none());
    }



}

