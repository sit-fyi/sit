//! Repository is where SIT stores all of its artifacts.
//!
//! It is represented by the [`Repository`] structure.
//!
//! [`Repository`]: struct.Repository.html
//!


use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;

use tempdir::TempDir;

use glob;

use serde_json;

use super::hash::HashingAlgorithm;
use super::encoding::Encoding;
#[cfg(feature = "deprecated-item-api")]
use super::id::IdGenerator;

use std::collections::HashMap;

/// Current repository format version
const VERSION: &str = "1";
/// Current repository features
const FEATURES: &[&str] = &[FEATURE_FLAT_RECORDS];
const FEATURE_FLAT_RECORDS: &str = "flat-records";

fn default_features() -> Vec<String> {
    FEATURES.iter().map(|s| s.to_string()).collect()
}
fn no_features() -> Vec<String> { vec![] }

/// Repository's config file name
const CONFIG_FILE: &str = "config.json";
/// Repository's issues path (deprecated)
const DEPRECATED_ISSUES_PATH: &str = "issues";
/// Repository's items path (deprecated)
const DEPRECATED_ITEMS_PATH: &str = "items";
/// Repository's items path
const RECORDS_PATH: &str = "records";
/// Repository's modules path
const MODULES_PATH: &str = "modules";


/// Repository is the container for all SIT artifacts
#[derive(Debug, Clone)]
pub struct Repository<MI> {
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
    #[cfg(feature = "deprecated-item-api")]
    items_path: PathBuf,
    /// Path to records. Mainly to avoid creating this path
    /// on demand for every operation that would require it
    records_path: PathBuf,
    /// Configuration
    config: Config,
    /// Module iterator
    module_iterator: MI,
    /// Integrity check
    integrity_check: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleDirectory<P: AsRef<Path>>(P);

pub trait ModuleIterator<P, E> {
    type Iter : Iterator<Item = Result<P, E>>;
    fn iter(&self) -> Result<Self::Iter, E>;
}

impl<P: AsRef<Path>> ModuleIterator<PathBuf, Error> for ModuleDirectory<P> {
    type Iter = ModuleDirectoryIterator;

    fn iter(&self) -> Result<Self::Iter, Error> {
        let path = self.0.as_ref();
        if !path.is_dir() {
            Ok(ModuleDirectoryIterator(None))
        } else {
            Ok(ModuleDirectoryIterator(Some(fs::read_dir(path)?)))
        }
    }
}

impl<T1, T2, P, E> ModuleIterator<P, E> for (T1, T2)
    where T1: ModuleIterator<P, E>, T2: ModuleIterator<P, E> {
    type Iter = std::iter::Chain<T1::Iter, T2::Iter>;

    fn iter(&self) -> Result<Self::Iter, E> {
        let t1 = self.0.iter()?;
        let t2 = self.1.iter()?;
        Ok(t1.chain(t2))
    }
}

pub struct ModuleDirectoryIterator(Option<fs::ReadDir>);

use crate::path::ResolvePath;

impl Iterator for ModuleDirectoryIterator {
    type Item = Result<PathBuf, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            None => None,
            Some(ref mut modules) => {
                match modules.next() {
                    None => None,
                    Some(Ok(f)) => Some(f.path().resolve_dir("/").map_err(|e| e.into())),
                    Some(Err(e)) => Some(Err(e.into())),
                }
            }
        }
    }
}

/// Repository configuration
#[derive(Debug, Clone, TypedBuilder, Serialize, Deserialize)]
pub struct Config {
     /// Hashing algorithm used
    hashing_algorithm: HashingAlgorithm,
    /// Encoding used
    encoding: Encoding,
    /// ID generator
    ///
    #[cfg(feature = "deprecated-item-api")]
    id_generator: IdGenerator,
    /// Repository version
    #[default = "String::from(VERSION)"]
    version: String,
    #[default = "default_features()"]
    #[serde(default = "no_features")]
    features: Vec<String>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

impl Config {
    /// Returns hashing algorithm
    pub fn hashing_algorithm(&self) -> &HashingAlgorithm {
        &self.hashing_algorithm
    }
    /// Returns encoding
    pub fn encoding(&self) -> &Encoding {
        &self.encoding
    }
    /// Returns extra configuration
    pub fn extra(&self) -> &HashMap<String, serde_json::Value> {
        &self.extra
    }
    /// Sets extra free-form properties in the configuration file
    /// (overrides existing ones)
    pub fn set_extra_properties<E, K, V>(&mut self, extra: E)
        where E: IntoIterator<Item = (K, V)>, K: AsRef<str>, V: Into<serde_json::Value> {
        for (k, v) in extra.into_iter() {
            self.extra.insert(k.as_ref().into(), v.into());
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Upgrade {
    IssuesToItems,
    ItemsToFlatRecords,
}

use std::fmt::{self, Display};

impl Display for Upgrade {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &Upgrade::IssuesToItems => write!(f, "renaming issues/ to items/"),
            &Upgrade::ItemsToFlatRecords => write!(f, "migrating items' records to flat records namespace"),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    /// Item already exists
    AlreadyExists,
    /// Item not found
    NotFound,
    /// Path prefix error
    ///
    /// Currently, this is used when one attempts to create a record file outside of the record
    PathPrefixError,
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
    IoError(std::io::Error),
    /// JSON (de)serialization error
    SerializationError(serde_json::Error),
    /// Base decoding error
    BaseDecodeError(::data_encoding::DecodeError),
    /// Other errors
    #[error(no_from, non_std)]
    OtherError(String),
}

impl From<glob::PatternError> for Error {
    fn from(err: glob::PatternError) -> Self {
        use std::error::Error as StandardError;
        Error::OtherError(format!("glob pattern error: {}", err.description()))
    }
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

use crate::path::HasPath;

impl Repository<ModuleDirectory<PathBuf>> {
    /// Attempts creating a new repository. Fails with `Error::AlreadyExists`
    /// if a repository already exists.
    pub fn new<P: Into<PathBuf>>(path: P) -> Result<Self, Error> {
        Repository::new_with_config(path, Config {
            hashing_algorithm: Default::default(),
            encoding: Encoding::default(),
            #[cfg(feature = "deprecated-item-api")]
            id_generator: IdGenerator::default(),
            version: String::from(VERSION),
            extra: HashMap::new(),
            features: default_features(),
        })
    }

    /// Attempts creating a new repository with a specified config. Fails with `Error::AlreadyExists`
    /// if a repository already exists.
    pub fn new_with_config<P: Into<PathBuf>>(path: P, config: Config) -> Result<Self, Error> {
        let path: PathBuf = path.into();
        if path.is_dir() && fs::read_dir(&path)?.next().is_some() {
            Err(Error::AlreadyExists)
        } else {
            let mut config_path = path.clone();
            config_path.push(CONFIG_FILE);
            #[cfg(feature = "deprecated-item-api")] let items_path = {
                let items_path = path.join(DEPRECATED_ITEMS_PATH);
                items_path
            };
            let records_path = path.join(RECORDS_PATH);
            fs::create_dir_all(&records_path)?;
            let modules_path = path.join(MODULES_PATH);
            let module_iterator = ModuleDirectory(modules_path.clone());
            let repo = Repository {
                path,
                config_path,
                #[cfg(feature = "deprecated-item-api")]
                items_path,
                records_path,
                config,
                modules_path,
                module_iterator,
                integrity_check: true,
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
        let modules_path = path.join(MODULES_PATH);
        let items_path = path.join(DEPRECATED_ITEMS_PATH);
        let records_path = path.join(RECORDS_PATH);
        let file = fs::File::open(&config_path)?;
        let mut config: Config = serde_json::from_reader(file)?;
        let upgraded = {
            let mut _upgraded = false;
            // items -> issues migration
            {
                let issues_path = path.join(DEPRECATED_ISSUES_PATH);
                if issues_path.is_dir() && !items_path.is_dir() {
                    if upgrades.as_ref().contains(&Upgrade::IssuesToItems) {
                        fs::rename(&issues_path, &items_path)?;
                        _upgraded = true;
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
                        _upgraded = true;
                    } else {
                        return Err(Error::UpgradeRequired(Upgrade::IssuesToItems));
                    }
                }
            }

            // flat records namespace
            if upgrades.as_ref().contains(&Upgrade::ItemsToFlatRecords) {
                fs::create_dir_all(&records_path)?;
                let glob_pattern = format!("{}/*/*", items_path.to_str().unwrap());
                let glob = glob::glob(&glob_pattern)?;
                for path in glob.filter_map(Result::ok).filter(|p| p.is_dir()) {
                    let mut components = path.components();
                    let record = components.next_back().unwrap();
                    let split_path = crate::record::split_path(record.as_os_str().to_str().unwrap(), 2);
                    let mut split_path_ = split_path.clone();
                    split_path_.pop();
                    fs::create_dir_all(records_path.join(split_path_))?;
                    fs::rename(&path, records_path.join(&split_path))?;
                    let mut f = fs::File::create(&path)?;
                    f.write(format!("../../{}/{}", RECORDS_PATH, split_path.to_str().unwrap()).as_bytes())?;
                }
                config.version = VERSION.into();
                if config.features.iter().find(|f| f.as_str() == FEATURE_FLAT_RECORDS).is_none() {
                    config.features.push(FEATURE_FLAT_RECORDS.into());
                }
                _upgraded = true;
            } else {
                let record_as_dir_present = items_path.is_dir() && walkdir::WalkDir::new(&items_path).min_depth(2).max_depth(2)
                    .into_iter().filter_entry(|e| e.file_type().is_dir()).filter_map(Result::ok).next().is_some();
                if record_as_dir_present || config.features.iter().find(|f| f.as_str() == FEATURE_FLAT_RECORDS).is_none() {
                    return Err(Error::UpgradeRequired(Upgrade::ItemsToFlatRecords));
                }
            }
            _upgraded
        };
        if config.version != VERSION {
            return Err(Error::InvalidVersion { expected: String::from(VERSION), got: config.version });
        }
        let module_iterator = ModuleDirectory(modules_path.clone());
        let repository = Repository {
            path,
            config_path,
            #[cfg(feature = "deprecated-item-api")]
            items_path,
            records_path,
            config,
            modules_path,
            module_iterator,
            integrity_check: true,
        };
        if upgraded {
            repository.save()?;
        }
        Ok(repository)
    }

    /// Finds SIT repository in `path` or any of its parent directories, or within the same
    /// hierarchy under a sub-directory `dir` (often `".sit"` by convention)
    pub fn find_in_or_above<P: Into<PathBuf>, S: AsRef<str>>(dir: S, path: P) -> Option<PathBuf> {
        let mut path: PathBuf = path.into();
        let dir = dir.as_ref();
        path.push(dir);
        loop {
            match path.parent() {
                Some(parent) => match Repository::open(&parent) {
                    Ok(_) | Err(Error::UpgradeRequired(_)) => return Some(parent.into()),
                    _ => (),
                },
                _ => (),
            }
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
                match Repository::open(&path) {
                    Ok(_) | Err(Error::UpgradeRequired(_)) => break,
                    _ => return None,
                }
            }
        }
        Some(path)
    }

}

impl<'a, MI> HasPath for Repository<MI> {
    fn path(&self) -> &Path {
        self.path.as_path()
    }
}

impl<MI> Repository<MI> {
    /// Returns a new instance of this Repository with an additional module iterator
    /// chained to the existing one
    pub fn with_module_iterator<MI1>(self, module_iterator: MI1) -> Repository<(MI, MI1)> {
        Repository {
            path: self.path,
            config_path: self.config_path,
            modules_path: self.modules_path,
            #[cfg(feature = "deprecated-item-api")]
            items_path: self.items_path,
            records_path: self.records_path,
            config: self.config,
            module_iterator: (self.module_iterator, module_iterator),
            integrity_check: self.integrity_check,
        }
    }

    /// Returns a new instance of this Repository with a different module iterator
    pub fn with_new_module_iterator<MI1>(self, module_iterator: MI1) -> Repository<MI1> {
        Repository {
            path: self.path,
            config_path: self.config_path,
            modules_path: self.modules_path,
            #[cfg(feature = "deprecated-item-api")]
            items_path: self.items_path,
            records_path: self.records_path,
            config: self.config,
            module_iterator,
            integrity_check: self.integrity_check,
        }
    }

    /// Returns the status of integrity check
    pub fn integrity_check(&self) -> bool {
        self.integrity_check
    }


    /// Mutably changes the requirement for integrity check
    pub fn set_integrity_check(&mut self, value: bool) {
        self.integrity_check = value;
    }

    /// Creates a new instance of `Repository` with a changed requirement for integrity check
    pub fn with_integrity_check(self, value: bool) -> Self {
        Repository {
            path: self.path,
            config_path: self.config_path,
            modules_path: self.modules_path,
            #[cfg(feature = "deprecated-item-api")]
            items_path: self.items_path,
            records_path: self.records_path,
            config: self.config,
            module_iterator: self.module_iterator,
            integrity_check: value,
        }
    }


    /// Saves the repository. Ensures the directory exists and the configuration has
    /// been saved.
    pub fn save(&self) -> Result<(), Error> {
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

    /// Returns items path
    #[cfg(feature = "deprecated-item-api")]
    pub fn items_path(&self) -> &Path {
        self.items_path.as_path()
    }

    /// Returns records path
    pub fn records_path(&self) -> &Path {
        self.records_path.as_path()
    }

    /// Returns repository's config
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns repository's mutable config
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    #[cfg(feature = "deprecated-item-api")]
    /// Returns an unordered (as in "order not defined") item iterator
    pub fn item_iter(&self) -> Result<ItemIter<MI>, Error> {
        fs::create_dir_all(self.items_path())?;
        Ok(ItemIter { repository: self, dir: fs::read_dir(&self.items_path)?, integrity_check: self.integrity_check })
    }

    #[cfg(feature = "deprecated-item-api")]
    /// Creates and returns a new item with a unique ID
    pub fn new_item(&self) -> Result<Item<MI>, Error> {
        self.new_named_item(self.config.id_generator.generate())
    }

    /// Creates and returns a new item with a specific name. Will fail
    /// if there's an item with the same name.
    #[cfg(feature = "deprecated-item-api")]
    pub fn new_named_item<S: Into<String>>(&self, name: S) -> Result<Item<MI>, Error> {
        fs::create_dir_all(self.items_path())?;
        let id: String = name.into();
        let path = self.items_path().join(&id);
        fs::create_dir(&path)?;
        let id = OsString::from(id);
        Ok(Item {
            repository: self,
            integrity_check: self.integrity_check,
            path,
            id,
        })
    }

    /// Finds a record by name (if there is one)
    pub fn record<S: AsRef<str>>(&self, name: S) -> Option<Record> {
        let path = self.records_path().join(crate::record::split_path(name, 2));
        let path = path.resolve_dir(self.path()).unwrap_or(path);
        if path.is_dir() && path.strip_prefix(self.records_path()).is_ok() {
            let hash = self.config.encoding.decode(path.file_name().unwrap().to_str().unwrap().as_bytes());
            if hash.is_err() {
                return None
            }
            let record = Record {
                hash: hash.unwrap(),
                encoding: self.config.encoding.clone(),
                path,
                #[cfg(feature = "deprecated-item-api")]
                item: "".into(),
            };
            Some(record)
        } else {
            None
        }
    }

    /// Finds an item by name (if there is one)
    #[cfg(feature = "deprecated-item-api")]
    pub fn item<S: AsRef<str>>(&self, name: S) -> Option<Item<MI>> {
        let path = self.items_path().join(name.as_ref());
        if path.exists() && path.strip_prefix(self.items_path()).is_ok() {
            let mut test = path.clone();
            test.pop();
            if test != self.items_path() {
                return None;
            }
            let id = path.file_name().unwrap().to_os_string();
            let p = self.items_path().join(&id);
            let path = p.resolve_dir(self.path()).unwrap_or(p);
            let item = Item {
                repository: self,
                integrity_check: self.integrity_check,
                path,
                id,
            };
            Some(item)
        } else {
            None
        }
    }

    /// Returns path to modules. The target directory may not exist.
    pub fn modules_path(&self) -> &Path {
        &self.modules_path
    }

    pub fn new_record_in<'f, P: AsRef<Path>, F: File + 'f, I: Into<OrderedFiles<'f, F>>>(&self, path: P, files: I, link_parents: bool) ->
    Result<Record, Error> where F::Read: 'f {
        let tempdir = TempDir::new_in(&self.path, "sit")?;
        let mut hasher = self.config.hashing_algorithm.hasher();

        let files: OrderedFiles<F> = files.into();

        // Link parents if requested
        let files = if link_parents {
            let records = self.record_iter()?.last().unwrap_or(vec![]);
            let parents: OrderedFiles<_> = records.iter().map(|rec| (format!(".prev/{}", rec.encoded_hash()), &b""[..])).into();
            files + parents
        } else {
            files.boxed()
        };

        files.hash_and(&mut *hasher, |n| -> Result<fs::File, Error> {
            let path = RelativePath::new(n).normalize();
            if path.components().any(|c| match c {
                RelativeComponent::Normal(_) => false,
                _ => true,
            }) {
                return Err(Error::PathPrefixError);
            }
            let actual_path = path.to_path(tempdir.path());
            let mut dir = actual_path.clone();
            dir.pop();
            fs::create_dir_all(dir)?;
            let file = fs::File::create(actual_path)?;
            Ok(file)
        }, |mut f, c| -> Result<fs::File, Error> { f.write(c).map(|_| f).map_err(|e| e.into()) })?;


        let hash = hasher.result_box();
        let path = path.as_ref().join(crate::record::split_path(self.config.encoding.encode(&hash), 2));
        if path.exists() {
            fs::remove_dir_all(tempdir.into_path())?;
        } else {
            if cfg!(windows) {
                // We have to handle Windows separately here because of how renaming works differently
                // on Windows. From `std::fs::rename` documentation:
                //
                //     This function currently corresponds to the `rename` function on Unix
                //     and the `MoveFileEx` function with the `MOVEFILE_REPLACE_EXISTING` flag on Windows.
                //
                //     Because of this, the behavior when both `from` and `to` exist differs. On
                //     Unix, if `from` is a directory, `to` must also be an (empty) directory. If
                //     `from` is not a directory, `to` must also be not a directory. In contrast,
                //     on Windows, `from` can be anything, but `to` must *not* be a directory.
                //
                // So, we are avoiding creating the last directory component in the path on Windows:
                fs::create_dir_all(path.parent().unwrap())?;
            } else {
                fs::create_dir_all(&path)?;
            }
            fs::rename(tempdir.into_path(), &path)?;
        }
        Ok(Record {
            hash,
            #[cfg(feature = "deprecated-item-api")]
            item: "".into(),
            path,
            encoding: self.config.encoding.clone(),
        })
    }
}

impl<MI> RecordOwningContainer for Repository<MI> {

    fn new_record<'f, F: File + 'f, I: Into<OrderedFiles<'f, F>>>(&self, files: I, link_parents: bool) -> Result<Record, Error> where F::Read: 'f {
        self.new_record_in(&self.records_path, files, link_parents)
    }
}


impl<MI> Repository<MI> where MI: ModuleIterator<PathBuf, Error>
{
    /// Returns an iterator over the list of modules (directories under `modules` directory)
    pub fn module_iter<'a>(&'a self) -> Result<MI::Iter, Error> {
        Ok(self.module_iterator.iter()?)
    }
}

use crate::record::RecordContainerReduction;
impl<MI> RecordContainerReduction for Repository<MI> { }

impl<MI> RecordContainer for Repository<MI> {
    type Error = Error;
    type Record = Record;
    type Records = Vec<Record>;
    type Iter = RepositoryRecordIterator;

    fn record_iter(&self) -> Result<Self::Iter, Self::Error> {
        let path = self.records_path().resolve_dir(self.path()).unwrap_or(self.records_path().into());
        let iter = GenericRecordIterator::new(self.config.hashing_algorithm.clone(),
                                              self.config.encoding.clone(),
                                              path,
                                              None,
                                              self.path().into());
        Ok(RepositoryRecordIterator {
            iter,
            integrity_check: self.integrity_check,
        })
    }

}


pub struct RepositoryRecordIterator {
    iter: GenericRecordIterator,
    integrity_check: bool,
}

impl Iterator for RepositoryRecordIterator {
    type Item = Vec<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|vec| {
            vec.into_iter().map(|(path, hash)|
                Record {
                    hash,
                    #[cfg(feature = "deprecated-item-api")]
                    item: "".into(),
                    path,
                    encoding: self.iter.encoding.clone(),
            }).filter(|r| self.integrity_check == false || r.integrity_intact(&self.iter.hashing_algorithm)).collect() }
        )
    }

}

impl<MI> PartialEq for Repository<MI> {
    fn eq(&self, rhs: &Repository<MI>) -> bool {
        (self as *const Repository<MI>) == (rhs as *const Repository<MI>)
    }
}

#[cfg(feature = "deprecated-item-api")]
use super::Item as ItemTrait;
#[cfg(feature = "deprecated-item-api")]
use std::ffi::OsString;

/// An item residing in a repository
#[derive(Debug, PartialEq)]
#[cfg(feature = "deprecated-item-api")]
pub struct Item<'a, MI: 'a> {
    repository: &'a Repository<MI>,
    id: OsString,
    integrity_check: bool,
    path: PathBuf,
}

use crate::record::{File, OrderedFiles};
use relative_path::{RelativePath, Component as RelativeComponent};

#[cfg(feature = "deprecated-item-api")]
impl<'a, MI: 'a> HasPath for Item<'a, MI> {
    fn path(&self) -> &Path {
        self.path.as_path()
    }
}

#[cfg(feature = "deprecated-item-api")]
impl<'a, MI: 'a> Item<'a, MI> {

    /// Returns the status of integrity check
    pub fn integrity_check(&self) -> bool {
        self.integrity_check
    }


    /// Mutably changes the requirement for integrity check
    pub fn set_integrity_check(&mut self, value: bool) {
        self.integrity_check = value;
    }

    /// Creates a new instance of `Item` with a changed requirement for integrity check
    pub fn with_integrity_check(self, value: bool) -> Self {
        Item {
            repository: self.repository,
            id: self.id,
            integrity_check: value,
            path: self.path,
        }
    }

    pub fn new_record_in<'f, P: AsRef<Path>, F: File + 'f, I: Into<OrderedFiles<'f, F>>>(&self, path: P, files: I, link_parents: bool) ->
           Result<<Item<'a, MI> as RecordContainer>::Record, <Item<'a, MI> as RecordContainer>::Error> where F::Read: 'f {
        self.repository.new_record_in(path, files, link_parents)
    }

}

use crate::record::RecordContainer;

#[cfg(feature = "deprecated-item-api")]
impl<'a, MI: 'a> RecordContainer for Item<'a, MI> {
    type Error = Error;
    type Record = Record;
    type Records = Vec<Record>;
    type Iter = ItemRecordIterator;

    fn record_iter(&self) -> Result<Self::Iter, Self::Error> {
        let path = self.path().resolve_dir(self.repository.path()).unwrap_or(self.path().into());
        let iter = GenericRecordIterator::new(self.repository.config.hashing_algorithm.clone(),
                                              self.repository.config.encoding.clone(),
                                              path,
                                              Some(1), self.repository.path().into());
        Ok(ItemRecordIterator {
            iter,
            item: self.id.clone(),
            integrity_check: self.integrity_check,
        })
    }

}

use crate::record::RecordOwningContainer;
#[cfg(feature = "deprecated-item-api")]
impl<'a, MI: 'a> RecordOwningContainer for Item<'a, MI> {

     fn new_record<'f, F: File + 'f, I: Into<OrderedFiles<'f, F>>>(&self, files: I, link_parents: bool) -> Result<Self::Record, Self::Error> where F::Read: 'f {
        let record = self.new_record_in(&self.repository.records_path, files, link_parents)?;
        // TODO: should we remove the record if creating a link file failed?
        let path = self.repository.items_path.join(self.id());
        fs::create_dir_all(&path)?;
        let record_path = crate::record::split_path(record.encoded_hash(), 2);
        let record_path_s = record_path.to_str().unwrap();
        #[cfg(windows)] // replace backslashes with slashes
        let record_path_s = record_path_s.replace("\\", "/");
        let mut f = fs::File::create(path.join(record.encoded_hash()))?;
        f.write(format!("../../{}/{}", RECORDS_PATH, record_path_s).as_bytes())?;
        Ok(record)
    }

}

#[cfg(feature = "deprecated-item-api")]
impl<'a, MI: 'a> ItemTrait for Item<'a, MI> {

    fn id(&self) -> &str {
        self.id.to_str().unwrap()
    }

}


#[cfg(feature = "deprecated-item-api")]
pub struct ItemRecordIterator {
    iter: GenericRecordIterator,
    item: OsString,
    integrity_check: bool,
}

#[cfg(feature = "deprecated-item-api")]
impl Iterator for ItemRecordIterator {
    type Item = Vec<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|vec| {
            vec.into_iter().map(|(path, hash)|
                Record {
                    hash,
                    item: self.item.clone(),
                    path,
                    encoding: self.iter.encoding.clone(),
            })
                .filter(|r| self.integrity_check == false || r.integrity_intact(&self.iter.hashing_algorithm))
                .collect() }
        )
    }

}

use walkdir;

/// An iterator over records
struct GenericRecordIterator {
    hashing_algorithm: HashingAlgorithm,
    encoding: Encoding,
    path: PathBuf,
    dir: Vec<walkdir::DirEntry>,
    parents: Vec<String>,
    root: PathBuf,
}

impl GenericRecordIterator {
    fn new(hashing_algorithm: HashingAlgorithm, encoding: Encoding, path: PathBuf,
           depth: Option<usize>, root: PathBuf) -> Self {
        let depth = depth.or_else(|| {
            let mut depth = hashing_algorithm.len() * 4 / encoding.bit_width();
            if hashing_algorithm.len() * 4 % encoding.bit_width() != 0 {
                depth +=1;
            }
            Some(depth)
        }).unwrap();
        let dir: Vec<_> = walkdir::WalkDir::new(&path).min_depth(depth).max_depth(depth)
            .into_iter().filter_map(Result::ok).collect();
        GenericRecordIterator {
            encoding,
            hashing_algorithm,
            dir,
            path,
            parents: vec![],
            root,
        }
    }
}

impl Iterator for GenericRecordIterator {
    type Item = Vec<(PathBuf, Vec<u8>)>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: if https://github.com/rust-lang/rust/issues/43244 is finalized, try to use drain_filter instead
        let (filtered, dir): (Vec<_>, Vec<_>) = std::mem::replace(&mut self.dir, vec![]).into_iter()
            .partition(|e| {
                let path = e.path().resolve_dir(&self.root).unwrap_or(e.path().to_path_buf());
                if !path.is_dir() {
                    return false
                }

                let name = e.file_name().to_str().unwrap();

                let valid_name = self.encoding.decode(name.as_bytes()).is_ok();
                if !valid_name {
                    return false;
                }

                let dot_prev = path.join(".prev");
                let has_all_valid_parents = !dot_prev.is_dir() || match fs::read_dir(dot_prev) {
                    Err(_) => false,
                    Ok(dir) => {
                        dir.filter_map(Result::ok)
                            // only use links pointing to actual directories
                            .filter(|l| {
                                #[cfg(feature ="deprecated-item-api")]
                                let is_dir = {
                                    let p = self.path.join(l.file_name().to_str().unwrap());
                                    p.resolve_dir(&self.root).unwrap_or(p).is_dir()
                                };
                                #[cfg(not(feature ="deprecated-item-api"))]
                                let is_dir = false;
                                is_dir || {
                                    let p = self.path.join(crate::record::split_path(l.file_name().to_str().unwrap(), 2));
                                    p.resolve_dir(&self.root).unwrap_or(p).is_dir()
                                }
                            })
                            // has to be already processed
                            .all(|l| self.parents.iter().any(|p| p.as_str() == l.file_name().to_str().unwrap()))
                    }
                };

                has_all_valid_parents
            });
        let result: Vec<_> = filtered.iter()
            .map(|e| {
                let name = e.file_name().to_str().unwrap();
                let decoded_name = self.encoding.decode(name.as_bytes()).unwrap();
                (e.path().resolve_dir(&self.root).unwrap_or(e.path().to_path_buf()), decoded_name)
            }).collect();
        self.dir = dir;
        if result.len() == 0 {
            return None
        }
        let encoding = self.encoding.clone();
        self.parents.append(&mut result.iter().map(|(_, rhash)| encoding.encode(rhash)).collect());
        Some(result)
    }
}


/// Unordered (as in "order not defined') item iterator
/// within a repository
#[cfg(feature = "deprecated-item-api")]
pub struct ItemIter<'a, MI: 'a> {
    repository: &'a Repository<MI>,
    dir: fs::ReadDir,
    integrity_check: bool,
}

#[cfg(feature = "deprecated-item-api")]
impl<'a, MI: 'a> Iterator for ItemIter<'a, MI> {
    type Item = Item<'a, MI>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.dir.next() {
                None => return None,
                // bail on an entry if the entry is erroneous
                Some(Err(_)) => continue,
                Some(Ok(entry)) => {
                    let p = self.repository.items_path().join(entry.file_name());
                    let path = p.resolve_dir(self.repository.path()).unwrap_or(p);
                    return Some(Item {
                        repository: self.repository,
                        id: entry.file_name(),
                        integrity_check: self.integrity_check,
                        path,
                    });
                }
            }
        }
    }
}

use super::Record as RecordTrait;

/// A record
#[derive(Debug, Clone)]
pub struct Record {
    hash: Vec<u8>,
    #[cfg(feature = "deprecated-item-api")]
    item: OsString,
    encoding: Encoding,
    path: PathBuf,
}

impl HasPath for Record {

    /// Returns path to the record
    fn path(&self) -> &Path {
        self.path.as_path()
    }

}


use serde::{Serialize, Serializer};

impl Serialize for Record {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        use crate::record::RecordExt;
        self.serde_serialize(serializer)
    }
}


impl PartialEq for Record {
   fn eq(&self, other: &Record) -> bool {
       self.hash == other.hash
   }
}

impl RecordTrait for Record {
    type Read = std::fs::File;
    type Str = String;
    type Hash = Vec<u8>;
    type Iter = RecordFileIterator;

    fn hash(&self) -> Self::Hash {
        self.hash.clone()
    }

    fn encoded_hash(&self) -> Self::Str {
        self.encoding.encode(&self.hash)
    }

    fn file_iter(&self) -> Self::Iter {
        let path = self.path();
        let glob_pattern = format!("{}/**/*", path.to_str().unwrap());
        RecordFileIterator {
            glob: glob::glob(&glob_pattern).expect("invalid glob pattern"),
            prefix: self.path().into(),
        }
    }
    #[cfg(feature = "deprecated-item-api")]
    fn item_id(&self) -> Self::Str {
        self.item.clone().into_string().unwrap()
    }
}

/// An iterator over files in a record
pub struct RecordFileIterator {
    glob: glob::Paths,
    prefix: PathBuf,
}

impl Iterator for RecordFileIterator {
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
    use crate::path::HasPath;

    #[test]
    fn new_repo() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        #[cfg(feature = "deprecated-item-api")] {
            assert_eq!(repo.item_iter().unwrap().count(), 0); // no items in a new repo
        }
        assert_eq!(repo.record_iter().unwrap().count(), 0); // no records in a new repo
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
    #[cfg(feature = "deprecated-item-api")]
    fn repo_persists_items() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let item = repo.new_item().unwrap();
        let repo = Repository::open(&tmp).unwrap();
        // load items
        let mut items: Vec<Item<_>> = repo.item_iter().unwrap().collect();
        assert_eq!(items.len(), 1);
        // check equality of the item's ID
        assert_eq!(items.pop().unwrap().id(), item.id());
    }

    #[test]
    fn repo_records_records() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create a record
        let record = repo.new_record(vec![("test/it", &[1u8][..])].into_iter(), false).unwrap();
        let repo = Repository::open(&tmp).unwrap();
        // load records
        let mut records: Vec<Vec<_>> = repo.record_iter().unwrap().collect();
        assert_eq!(records.len(), 1);
        // check equality of the item's ID
        assert_eq!(records.pop().unwrap().pop().unwrap().hash(), record.hash());
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
        // non-repo shouldn't be found
        let tmp1 = TempDir::new("sit").unwrap().into_path() ;
        let non_sit = tmp1.join(".sit");
        fs::create_dir_all(non_sit).unwrap();
        let deep_subdir = tmp1.join("a/b/c/d");
        assert!(Repository::find_in_or_above(".sit", &deep_subdir).is_none());
    }

    #[test]
    fn find_upgradable_repo() {
        let tmp = TempDir::new("sit").unwrap().into_path();
        let sit = tmp.join(".sit");
        // create repo w/o flat-records
        let mut repo = Repository::new(&sit).unwrap();
        repo.config.features = vec![];
        repo.save().unwrap();
        let deep_subdir = tmp.join("a/b/c/d");
        let repo = Repository::find_in_or_above(".sit", &deep_subdir);
        assert!(repo.is_some());
    }

    #[test]
    fn find_repo_in_itself() {
        // unlike `find_repo`, this tests whether we can find a repository
        // that is not contained in a `.sit` folder
        let sit = TempDir::new("sit").unwrap().into_path();
        // create repo
        Repository::new(&sit).unwrap();
        let subdir = sit.join("items");
        let repo = Repository::find_in_or_above(".sit", &subdir);
        assert!(repo.is_some());
        let repo = Repository::open(repo.unwrap()).unwrap();
        assert_eq!(repo.path(), sit);
    }


    #[test]
    #[cfg(feature = "deprecated-item-api")]
    fn new_item() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let item = repo.new_item().unwrap();
        // load items
        let mut items: Vec<Item<_>> = repo.item_iter().unwrap().collect();
        assert_eq!(items.len(), 1);
        // check equality of the item's ID
        assert_eq!(items.pop().unwrap().id(), item.id());
    }

    #[test]
    #[cfg(feature = "deprecated-item-api")]
    fn new_named_item() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let item = repo.new_named_item("one").unwrap();
        // load items
        let mut items: Vec<Item<_>> = repo.item_iter().unwrap().collect();
        assert_eq!(items.len(), 1);
        // check equality of the item's ID
        assert_eq!(items.pop().unwrap().id(), item.id());
    }

    #[test]
    #[cfg(feature = "deprecated-item-api")]
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
    #[cfg(feature = "deprecated-item-api")]
    fn find_item() {
         let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let item = repo.new_named_item("one").unwrap();
       // find an existing item
        assert_eq!(repo.item("one").unwrap(), item);
        // find a non-existing item
        assert!(repo.item("two").is_none());
        // point outside of items
        assert!(repo.item("/").is_none());
        // point anywhere not one level below items
        assert!(repo.item("one/..").is_none());
        item.new_record(vec![("test/it", &[1u8][..])].into_iter(), false).unwrap();
        assert!(repo.item("one/it").is_none());
    }

    /// This test ensures that item symlinks expressed as text files (for system
    /// without symlinks) will be interpreted as symlinks
    #[test]
    #[cfg(feature = "deprecated-item-api")]
    fn item_path_link() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create an item
        let item = repo.new_item().unwrap();
        // move the item
        fs::rename(item.path(), tmp.join(item.id())).unwrap();
        // link it
        let mut f = fs::File::create(repo.items_path().join(item.id())).unwrap();
        f.write(format!("../{}", item.id()).as_bytes()).unwrap();
        use dunce;
        // find it
        assert_eq!(dunce::canonicalize(repo.item(item.id()).unwrap().path()).unwrap(),
                   dunce::canonicalize(tmp.join(item.id())).unwrap());
        // iterate for it
        let mut item_iter = repo.item_iter().unwrap();
        assert_eq!(item_iter.next().unwrap().id(), item.id());
        assert!(item_iter.next().is_none());
    }

    /// This test ensures that record symlinks expressed as text files (for system
    /// without symlinks) will be interpreted as symlinks
    #[test]
    fn record_path_link() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create a record
        let record = repo.new_record(vec![("test", &b"hello"[..])].into_iter(), true).unwrap();
        // move the record
        fs::rename(record.path(), tmp.join(record.encoded_hash())).unwrap();
        // link it
        let mut f = fs::File::create(repo.records_path().join(record.split_path(2))).unwrap();
        f.write(format!("../../../../../../../../../../../../../../../../{}",
                        record.encoded_hash()).as_bytes()).unwrap();
        println!("{:?}", repo.path());
        // iterate for it
        let mut record_iter = repo.record_iter().unwrap();
        assert_eq!(record_iter.next().unwrap().get(0).unwrap().encoded_hash(), record.encoded_hash());
        assert!(record_iter.next().is_none());
    }


    #[test]
    #[cfg(feature = "deprecated-item-api")]
    fn new_item_record() {
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
    fn new_record() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create a record
        let record = repo.new_record(vec![("test", &b"hello"[..])].into_iter(), true).unwrap();
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
        let mut records: Vec<Record> = repo.record_iter().unwrap().flat_map(|v| v).collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records.pop().unwrap().hash(), record.hash());
        // find record
        assert_eq!(repo.record(record.encoded_hash()).unwrap().hash(), record.hash());
    }

    #[test]
    fn record_split_path() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create a record
        let record = repo.new_record(vec![("test", &b"hello"[..])].into_iter(), true).unwrap();
        let path = record.split_path(2);
        assert_eq!(path, Path::new("7T/VN/NJ/XP/VZ/PZ/FJ/W5/WO/UI/FY/CR/2X/TN/5T/7TVNNJXPVZPZFJW5WOUIFYCR2XTN5TNG"));
        assert_eq!(record.path().strip_prefix(repo.records_path()).unwrap(), path);
    }

    #[test]
    fn record_integrity_check() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create a record
        let record = repo.new_record(vec![("test", &b"hello"[..])].into_iter(), true).unwrap();
        // tamper with the record
        let mut file = fs::File::create(record.path().join("file")).unwrap();
        file.write(b"test").unwrap();
        drop(file);
        // list records
        let records: Vec<Record> = repo.record_iter().unwrap().flat_map(|v| v).collect();
        // invalid record should not be listed
        assert_eq!(records.len(), 0);
        // disable integrity check
        let repo = repo.clone().with_integrity_check(false);
        let mut records: Vec<Record> = repo.record_iter().unwrap().flat_map(|v| v).collect();
        // now there should be a record
        assert_eq!(records.len(), 1);
        assert_eq!(records.pop().unwrap().hash(), record.hash());
    }

    #[test]
    #[cfg(feature = "deprecated-item-api")]
    fn item_record_integrity_check_propagates_from_repository() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let mut repo = Repository::new(&tmp).unwrap();
        repo.set_integrity_check(false);
        // create an item
        let item = repo.new_item().unwrap();
        assert!(!item.integrity_check());
        // create a record
        let record = item.new_record(vec![("test", &b"hello"[..])].into_iter(), true).unwrap();
        // tamper with the record
        let mut file = fs::File::create(repo.items_path().join(item.id()).join(record.encoded_hash()).resolve_dir(repo.path()).unwrap().join("file")).unwrap();
        file.write(b"test").unwrap();
        drop(file);
        // list records
        let mut records: Vec<Record> = item.record_iter().unwrap().flat_map(|v| v).collect();
        // now there should be a record
        assert_eq!(records.len(), 1);
        assert_eq!(records.pop().unwrap().hash(), record.hash());
    }

    #[test]
    fn record_integrity_check_propagates_from_repository() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let mut repo = Repository::new(&tmp).unwrap();
        repo.set_integrity_check(false);
        // create a record
        let record = repo.new_record(vec![("test", &b"hello"[..])].into_iter(), true).unwrap();
        // tamper with the record
        let mut file = fs::File::create(repo.records_path().join(record.split_path(2)).resolve_dir(repo.path()).unwrap().join("file")).unwrap();
        file.write(b"test").unwrap();
        drop(file);
        // list records
        let mut records: Vec<Record> = repo.record_iter().unwrap().flat_map(|v| v).collect();
        // now there should be a record
        assert_eq!(records.len(), 1);
        assert_eq!(records.pop().unwrap().hash(), record.hash());
    }

    #[test]
    fn record_files_path() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // attempt to create a record with an invalid filename
        assert_matches!(repo.new_record(vec![(".", &b"hello"[..])].into_iter(), false), Err(Error::IoError(_)));
        assert_matches!(repo.new_record(vec![("../test", &b"hello"[..])].into_iter(), false), Err(Error::PathPrefixError));
        assert_matches!(repo.new_record(vec![("something/../../test", &b"hello"[..])].into_iter(), false), Err(Error::PathPrefixError));
        // however, these are alright
        assert_matches!(repo.new_record(vec![("something/../test", &b"hello"[..])].into_iter(), false), Ok(_));
        assert_matches!(repo.new_record(vec![("./test1", &b"hello"[..])].into_iter(), false), Ok(_));
        // root is normalized, too
        let record = repo.new_record(vec![("/test2", &b"hello"[..])].into_iter(), false).unwrap();
        assert_eq!(record.file_iter().next().unwrap().name(), "test2");
    }

    #[test]
    fn new_record_parents_linking() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create a few top records
        let record1 = repo.new_record(vec![("test", &[1u8][..])].into_iter(), false).unwrap();
        let record1link = format!(".prev/{}", record1.encoded_hash());
        let record2 = repo.new_record(vec![("test", &[2u8][..])].into_iter(), false).unwrap();
        let record2link = format!(".prev/{}", record2.encoded_hash());
        // now attempt to create a record that should link both together
        let record = repo.new_record(vec![("test", &[3u8][..])].into_iter(), true).unwrap();
        assert!(record.file_iter().any(|(name, _)| name == *&record1link));
        assert!(record.file_iter().any(|(name, _)| name == *&record2link));
    }

    #[test]
    fn record_ordering() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create a few top records
        let record1 = repo.new_record(vec![("test", &[1u8][..])].into_iter(), false).unwrap();
        let record2 = repo.new_record(vec![("test", &[2u8][..])].into_iter(), false).unwrap();
        // now attempt to create a record that should link both together
        let record3 = repo.new_record(vec![("test", &[3u8][..])].into_iter(), true).unwrap();
        // and another top record
        let record4 = repo.new_record(vec![("test", &[4u8][..])].into_iter(), false).unwrap();
        // and another linking record
        let record5 = repo.new_record(vec![("test", &[5u8][..])].into_iter(), true).unwrap();

        // now, look at their ordering
        let mut records: Vec<_> = repo.record_iter().unwrap().collect();
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
    fn multilevel_parents() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();
        // create a top record
        let record1 = repo.new_record(vec![("test", &[1u8][..])].into_iter(), false).unwrap();
        // create a record right below it
        let record2 = repo.new_record(vec![("test", &[2u8][..])].into_iter(), true).unwrap();
        // now attempt to create a record that should link both together
        let record3 = repo.new_record(vec![("test", &[3u8][..]),
                                           (&format!(".prev/{}", record1.encoded_hash()), &[][..]),
                                           (&format!(".prev/{}", record2.encoded_hash()), &[][..]),
        ].into_iter(), false).unwrap();

        // now, look at their ordering
        let mut records: Vec<_> = repo.record_iter().unwrap().collect();
        let row_3 = records.pop().unwrap();
        let row_2 = records.pop().unwrap();
        let row_1 = records.pop().unwrap();
        assert_eq!(records.len(), 0);

        assert_eq!(row_1.len(), 1);
        assert!(row_1.iter().any(|r| r == &record1));

        assert_eq!(row_2.len(), 1);
        assert!(row_2.iter().any(|r| r == &record2));

        assert_eq!(row_3.len(), 1);
        assert!(row_3.iter().any(|r| r == &record3));
    }

    #[test]
    fn partial_ordering() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");

        // first repo
        let repo1 = Repository::new(&tmp).unwrap();
        // create a few top records
        let _record0 = repo1.new_record(vec![("test", &[2u8][..])].into_iter(), false).unwrap();
        // this record will link only to one top record
        let record1 = repo1.new_record(vec![("test", &[3u8][..])].into_iter(), true).unwrap();
        let record2 = repo1.new_record(vec![("test", &[1u8][..])].into_iter(), false).unwrap();
        // now attempt to create a record that should link both together
        let record3 = repo1.new_record(vec![("test", &[3u8][..])].into_iter(), true).unwrap();


        let mut tmp1 = TempDir::new("sit").unwrap().into_path();
        tmp1.push(".sit");

        // second repo
        let repo2 = Repository::new(&tmp1).unwrap();
        // replicate one of the top records only
        let record2_2 = repo2.new_record(record2.file_iter(), false).unwrap();

        // now copy record3 that linked both top records in the first repo
        // to the second repo
        let record3_2 = repo2.new_record(record3.file_iter(), false).unwrap();
        // ensure their hashes match
        assert_eq!(record3_2.hash(), record3.hash());

        // now copy record1 that linked both top records in the first repo
        // to the second repo
        let record1_2 = repo2.new_record(record1.file_iter(), false).unwrap();
        // ensure their hashes match
        assert_eq!(record1_2.hash(), record1.hash());

        // now, look at the records in the second item
        let mut records: Vec<_> = repo2.record_iter().unwrap().collect();
        let row_2 = records.pop().unwrap();
        let row_1 = records.pop().unwrap();
        assert_eq!(records.len(), 0);

        // ensure the partially resolvable record to be there
        assert_eq!(row_2.len(), 1);
        assert!(row_2.iter().any(|r| r == &record3_2));

        assert_eq!(row_1.len(), 2);
        // as well as one of its parents
        assert!(row_1.iter().any(|r| r == &record2_2));
        // as well as the one that has no resolvable parents
        assert!(row_1.iter().any(|r| r == &record1_2));
    }

    #[test]
    fn record_deterministic_hashing() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo1 = Repository::new(tmp.join("1")).unwrap();
        let repo2 = Repository::new(tmp.join("2")).unwrap();
        let repo3 = Repository::new(tmp.join("3")).unwrap();

        let record1 = repo1.new_record(vec![("z/a", &[2u8][..]), ("test", &[1u8][..])].into_iter(), false).unwrap();
        let record2 = repo2.new_record(vec![("test", &[1u8][..]), ("z/a", &[2u8][..])].into_iter(), false).unwrap();
        assert_eq!(record1.hash(), record2.hash());
        let record3 = repo3.new_record(vec![("test", &[1u8][..]), ("z\\a", &[2u8][..])].into_iter(), false).unwrap();
        assert_eq!(record3.hash(), record2.hash());
    }

    #[test]
    fn record_outside_naming_scheme() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let mut tmp1 = tmp.clone();
        tmp1.pop();

        let repo = Repository::new(&tmp).unwrap();
        let _record1 = repo.new_record(vec![("z/a", &[2u8][..]), ("test", &[1u8][..])].into_iter(), false).unwrap();
        let record2 = repo.new_record_in(&tmp1, vec![("a", &[2u8][..])].into_iter(), true).unwrap();

        // lets test that record2 can iterate over correct files
        let files: Vec<_> = record2.file_iter().collect();
        assert_eq!(files.len(), 2); // a and .prev/...


        // record2 can't be found as it is outside of the standard naming scheme
        let records: Vec<Vec<_>> = repo.record_iter().unwrap().collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].len(), 1);

        // On Windows, if a file within a directory that is being
        // moved is open (even for reading), this will prevent
        // this said directory from being moved, returning "Access Denied"
        // Therefore, we drop `files` here to release the `File` readers
        #[cfg(windows)]
        drop(files);

        fs::create_dir_all(repo.records_path().join(record2.split_path(2)).parent().unwrap()).unwrap();
        fs::rename(record2.path(), repo.records_path().join(record2.split_path(2))).unwrap();

        // and now it can be
        let records: Vec<Vec<_>> = repo.record_iter().unwrap().collect();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].len(), 1);
        assert_eq!(records[0].len(), 1);

    }

    #[test]
    #[cfg(feature = "deprecated-item-api")]
    fn issues_to_items_upgrade() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let mut tmp1 = tmp.clone();
        tmp1.pop();

        let mut repo = Repository::new(&tmp).unwrap();
        {
           repo.config.features = vec![];
        }
        repo.save().unwrap();
        let _item = repo.new_item().unwrap();
        assert_eq!(repo.item_iter().unwrap().count(), 1);


        std::fs::rename(repo.items_path(), repo.path().join("issues")).unwrap();
        let repo = Repository::open(&tmp);
        assert!(repo.is_err());
        assert_matches!(repo.unwrap_err(), Error::UpgradeRequired(Upgrade::IssuesToItems));

        let mut repo = Repository::open_and_upgrade(&tmp, &[Upgrade::IssuesToItems, Upgrade::ItemsToFlatRecords]).unwrap();
        assert!(!repo.path().join("issues").exists());
        assert_eq!(repo.item_iter().unwrap().count(), 1);

        // now, a more complicated case:
        // both issues/ and items/ are present
        // this can happen when merging a patch that changes .sit/issues
        // (prepared before the migration)
        {
            repo.config.features = vec![];
        }
        repo.save().unwrap();

        let item = repo.new_item().unwrap();
        std::fs::create_dir_all(repo.path().join("issues")).unwrap();
        std::fs::rename(repo.items_path().join(item.id()), repo.path().join("issues").join(item.id())).unwrap();


        let repo = Repository::open(&tmp);
        assert!(repo.is_err());
        assert_matches!(repo.unwrap_err(), Error::UpgradeRequired(Upgrade::IssuesToItems));

        let repo = Repository::open_and_upgrade(&tmp, &[Upgrade::IssuesToItems, Upgrade::ItemsToFlatRecords]).unwrap();
        assert!(!repo.path().join("issues").exists());
        assert_eq!(repo.item_iter().unwrap().count(), 2);

    }

    #[test]
    #[cfg(feature = "deprecated-item-api")]
    fn layout_v2_upgrade() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let mut tmp1 = tmp.clone();
        tmp1.pop();

        let mut repo = Repository::new(&tmp).unwrap();
        {
           repo.config.features = vec![];
        }
        repo.save().unwrap();
        let item = repo.new_item().unwrap();
        assert_eq!(repo.item_iter().unwrap().count(), 1);
        let record = item.new_record(vec![("test", &[0u8][..])].into_iter(), false).unwrap();

        // move the record back to items/
        fs::remove_file(item.path().join(record.encoded_hash())).unwrap();
        fs::rename(record.path(), item.path().join(record.encoded_hash())).unwrap();
        // the record can still be found, but not in records/
        let record1 = item.record_iter().unwrap().next().unwrap().pop().unwrap();
        assert_eq!(record1.path(), item.path().join(record1.encoded_hash()));

        let repo = Repository::open(&tmp);
        assert!(repo.is_err());
        assert_matches!(repo.unwrap_err(), Error::UpgradeRequired(Upgrade::ItemsToFlatRecords));

        let repo = Repository::open_and_upgrade(&tmp, &[Upgrade::ItemsToFlatRecords]).unwrap();
        let item = repo.item(item.id()).unwrap();
        let record1 = item.record_iter().unwrap().next().unwrap().pop().unwrap();
        // Record is back where it belongs
        use dunce;
        assert_eq!(dunce::canonicalize(record1.path()).unwrap(), dunce::canonicalize(repo.records_path().join(crate::record::split_path(record1.encoded_hash(), 2))).unwrap());

        // In a decentralized scenarios, some v1 updates might come at a point past the upgrade,
        // meaning v1 items will be injected into v2 repositories (because of delays or somebody
        // using an older version of SIT)
        // We need to ensure that the items will be continuously migrated.

        let new_item = repo.new_item().unwrap();
        let new_record = new_item.new_record(vec![("test", &[1u8][..])].into_iter(), false).unwrap();

        // move the record back to items/
        fs::remove_file(new_item.path().join(new_record.encoded_hash())).unwrap();
        fs::rename(new_record.path(), new_item.path().join(new_record.encoded_hash())).unwrap();

        let repo = Repository::open(&tmp);
        assert!(repo.is_err());
        assert_matches!(repo.unwrap_err(), Error::UpgradeRequired(Upgrade::ItemsToFlatRecords));

        let repo = Repository::open_and_upgrade(&tmp, &[Upgrade::ItemsToFlatRecords]).unwrap();
        let new_record_2 = repo.record(new_record.encoded_hash()).unwrap();

        assert_eq!(new_record.hash(), new_record_2.hash());

        let new_item_2 = repo.item(new_item.id()).unwrap();
        let new_record_2_1 = new_item_2.record_iter().unwrap().next().unwrap().pop().unwrap();

        assert_eq!(new_record_2_1.hash(), new_record_2.hash());
    }
    
    #[test]
    fn fixed_roots() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(&tmp).unwrap();

        // create two top records
        let record1 = repo.new_record(vec![("test", &[1u8][..])].into_iter(), false).unwrap();
        let record2 = repo.new_record(vec![("test", &[2u8][..])].into_iter(), false).unwrap();
        // create a record referring only to one root
        let record3 = repo.new_record(vec![("test", &[3u8][..]),
                                           (&format!(".prev/{}", record1.encoded_hash()), &[][..]),
        ].into_iter(), false).unwrap();
        // create a record referring only to another root
        let record4 = repo.new_record(vec![("test", &[3u8][..]),
                                           (&format!(".prev/{}", record2.encoded_hash()), &[][..]),
        ].into_iter(), false).unwrap();

        let mut subset1 = repo.fixed_roots(vec![record1.encoded_hash()]).record_iter().unwrap();
        let first = subset1.next().unwrap();
        assert_eq!(first, vec![record1.clone()]);
        let next = subset1.next().unwrap();
        assert_eq!(next, vec![record3.clone()]);

        let mut subset2 = repo.fixed_roots(vec![record2.encoded_hash()]).record_iter().unwrap();
        let first = subset2.next().unwrap();
        assert_eq!(first, vec![record2.clone()]);
        let next = subset2.next().unwrap();
        assert_eq!(next, vec![record4.clone()]);

        // fixed roots, same level of roots
        let mut subset3 = repo.fixed_roots(vec![record1.encoded_hash(), record2.encoded_hash()]).record_iter().unwrap();
        let first = subset3.next().unwrap();
        assert_eq!(first.len(), 2);
        assert!(first.iter().any(|e| e.encoded_hash() == record1.encoded_hash()));
        assert!(first.iter().any(|e| e.encoded_hash() == record2.encoded_hash()));
        let next = subset3.next().unwrap();
        assert_eq!(next.len(), 2);
        assert!(next.iter().any(|e| e.encoded_hash() == record3.encoded_hash()));
        assert!(next.iter().any(|e| e.encoded_hash() == record4.encoded_hash()));

        // fixed roots, different levels of roots
        let mut subset4 = repo.fixed_roots(vec![record1.encoded_hash(), record4.encoded_hash()]).record_iter().unwrap();
        let first = subset4.next().unwrap();
        assert_eq!(first.len(), 1);
        assert!(first.iter().any(|e| e.encoded_hash() == record1.encoded_hash()));
        let next = subset4.next().unwrap();
        assert_eq!(next.len(), 2);
        assert!(next.iter().any(|e| e.encoded_hash() == record3.encoded_hash()));
        assert!(next.iter().any(|e| e.encoded_hash() == record4.encoded_hash()));
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

        assert_eq!(::dunce::canonicalize(iter.next().unwrap().unwrap()).unwrap(), ::dunce::canonicalize(path).unwrap());
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

        assert_eq!(iter.next().unwrap().unwrap(), tmp2);
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

        assert_eq!(::dunce::canonicalize(iter.next().unwrap().unwrap()).unwrap(), ::dunce::canonicalize(tmp.join("module1")).unwrap());
        assert!(iter.next().is_none());
    }

    #[test]
    fn chaining_module_iterator() {
        let tmp = TempDir::new("sit").unwrap().into_path();
        fs::create_dir_all(tmp.join("modules").join("1")).unwrap();
        fs::create_dir_all(tmp.join("modules_1").join("1")).unwrap();
        let mi1 = ModuleDirectory(tmp.join("modules"));
        let mi2 = ModuleDirectory(tmp.join("modules_1"));

        let mut iter = (mi1, mi2).iter().unwrap();
        assert_eq!(tmp.join("modules").join("1"), iter.next().unwrap().unwrap());
        assert_eq!(tmp.join("modules_1").join("1"), iter.next().unwrap().unwrap());
        assert!(iter.next().is_none());
    }

}

