//! Record is an immutable collection of files

use std::io::{self, Read};
use hash::Hasher;

/// Record's file
///
/// This trait represent an abstraction of a file: something that has a name
/// and binary content to read.
pub trait File {
    /// Associated `Read` type
    type Read : Read;
    /// Returns file's name
    fn name(&self) -> &str;
    /// Returns a mutable reference to `Self::Read`
    fn read(&mut self) -> &mut Self::Read;
    /// Consumes itself and returns `Self::Read`
    fn into_read(self) -> Self::Read;
}

impl<S, R> File for (S, R) where S: AsRef<str>, R: Read {
    type Read = R;

    fn name(&self) -> &str {
        self.0.as_ref()
    }

    fn read(&mut self) -> &mut Self::Read {
        &mut self.1
    }

    fn into_read(self) -> Self::Read {
        self.1
    }

}

use std::marker::PhantomData;

/// A collection of always ordered files
///
/// With limited ways to construct this structure, it's
/// always ensured to have all its files sorted as required
/// by SIT for deterministic hashing.
pub struct OrderedFiles<'a, F: File>(Vec<F>, PhantomData<&'a ()>);

impl<'a, F: File> OrderedFiles<'a, F>  where F: 'a, F::Read: 'a {
    /// Returns a boxed version of itself
    ///
    /// It's useful to ensure type compatibility between branches,
    /// in one of which an intersection of differently-typed `File`s
    /// were ordered together.
    ///
    /// ```
    /// extern crate sit_core;
    ///
    /// use std::io::Cursor;
    /// use sit_core::record::{BoxedOrderedFiles, OrderedFiles};
    ///
    /// let files: OrderedFiles<_> = vec![("file", &b"hello"[..])].into();
    /// let extra: OrderedFiles<_> =  vec![("file", Cursor::new(String::from("world")))].into();
    ///
    /// let some_condition = true;
    ///
    /// let all_files = if some_condition {
    ///    files + extra
    /// } else {
    ///    files.boxed()
    /// };
    ///
    /// ```
    pub fn boxed(self) -> BoxedOrderedFiles<'a> {
        #[inline]
        fn boxed_file<'f, F: File + 'f>(file: F) -> (String, Box<Read + 'f>) where F::Read: 'f {
            (file.name().into(), Box::new(file.into_read()) as Box<Read + 'f>)
        }
        let files: Vec<_> = self.0.into_iter().map(boxed_file).collect();
        files.into()
    }
}

impl<'a, F: File> OrderedFiles<'a, F> {
    /// Deterministically hashes all ordered files and allows to process them as well
    ///
    /// For every file, it will call `per_file(file_name)` and use the returned positive value
    /// (from inside of `Ok(f_)`) to call `per_chunk(f_, chunk)` on every chunk of read data.
    ///
    /// This method's primary motivation is to allow hashing and saving files at the same time,
    /// to avoid re-reading them to accomplish both of the operations. By itself, however,
    /// this function doesn't do anything in term of saving files (or any other functionality),
    /// that is responsibility of `per_file` and `per_chunk` callbacks.
    pub fn hash_and<PF, F_, PC>(mut self, hasher: &mut Hasher, per_file: PF, per_chunk: PC) -> Result<(), io::Error>
        where PF: Fn(&str) -> Result<F_, io::Error>, PC: Fn(F_, &[u8]) -> Result<F_, io::Error> {
        let mut buf = vec![0; 4096];
        for file in self.0.iter_mut() {
            #[cfg(windows)] // replace backslashes with slashes
            let name_for_hashing: String = file.name().replace("\\", "/").into();
            #[cfg(unix)]
            let name_for_hashing: String = file.name().into();
            hasher.process(name_for_hashing.as_bytes());
            let mut reader = file.read();
            let mut file_processor = per_file(&name_for_hashing)?;
            loop {
                let bytes_read = reader.read(&mut buf)?;
                hasher.process(&buf);
                file_processor = per_chunk(file_processor, &buf[0..bytes_read])?;
                if bytes_read == 0 {
                    break;
                }
            }
        }
        Ok(())
    }
    /// Deterministically hashes all ordered files
    pub fn hash(self, hasher: &mut Hasher) -> Result<(), io::Error> {
        self.hash_and(hasher, |_| Ok(()), |v, _| Ok(v))
    }
}

impl<'a, I, F> From<I> for OrderedFiles<'a, F> where I: IntoIterator<Item=F>, F: File + 'a {
    fn from(i: I) -> Self {
        let mut files: Vec<_> = i.into_iter().collect();
        files.sort_unstable_by(|f1, f2| f1.name().cmp(f2.name()));
        OrderedFiles(files, PhantomData)
    }
}

pub type BoxedOrderedFiles<'a> = OrderedFiles<'a, (String, Box<Read + 'a>)>;

use std::ops::{Add, Sub};

impl<'a, F1, F2> Add<OrderedFiles<'a, F2>> for OrderedFiles<'a, F1> where F1: File + 'a, F2: File + 'a, F1::Read: 'a, F2::Read: 'a {
    type Output = BoxedOrderedFiles<'a>;

    fn add(self, rhs: OrderedFiles<'a, F2>) -> Self::Output {
        let mut files = self.boxed().0;
        let mut rhs_files = rhs.boxed().0;
        files.append(&mut rhs_files);
        files.into()
    }
}

impl<'a, F1, F2, I> Add<I> for OrderedFiles<'a, F1> where F1: File + 'a, F2: File + 'a, F1::Read: 'a, F2::Read: 'a, I: IntoIterator<Item = OrderedFiles<'a, F2>> {
    type Output = BoxedOrderedFiles<'a>;

    fn add(self, rhs: I) -> Self::Output {
        let mut files = self.boxed().0;
        for rhs in rhs.into_iter() {
            let mut rhs_files = rhs.boxed().0;
            files.append(&mut rhs_files);
        }
        files.into()
    }
}

impl<'a, F, S> Sub<S> for OrderedFiles<'a, F> where F: File + 'a, S: AsRef<str> + 'a {
    type Output = Self;
    fn sub(self, rhs: S) -> Self::Output {
        let name = rhs.as_ref();
        let files: Vec<_> = self.0.into_iter().filter(|f| f.name() != name).collect();
        files.into()
    }
}

#[cfg(test)]
mod ordered_files_tests {
    use proptest::collection::*;
    use super::*;

    proptest! {
      #[test]
      fn sorted(ref i in vec("\\PC*", 0..10)) {
        let ordered_files = OrderedFiles::from(i.clone().into_iter().map(|v| (v, &[][..])));
        for i in 1..ordered_files.0.len() {
           assert!(ordered_files.0[i].name() >= ordered_files.0[i-1].name());
        }
      }

      #[test]
      fn add_sorted(ref i1 in vec("\\PC*", 0..10), ref i2 in vec("\\PC*", 0..10)) {
        let ordered_files1 = OrderedFiles::from(i1.clone().into_iter().map(|v| (v, &[][..])));
        let ordered_files2 = OrderedFiles::from(i2.clone().into_iter().map(|v| (v, &[][..])));
        let ordered_files = ordered_files1 + ordered_files2;
        for i in 1..ordered_files.0.len() {
           assert!(ordered_files.0[i].name() >= ordered_files.0[i-1].name());
        }
      }

      #[test]
      fn add_includes(ref i1 in vec("\\PC*", 0..10), ref i2 in vec("\\PC*", 0..10)) {
        let ordered_files1 = OrderedFiles::from(i1.clone().into_iter().map(|v| (v, &[][..])));
        let ordered_files1_ = OrderedFiles::from(i1.clone().into_iter().map(|v| (v, &[][..])));
        let ordered_files2 = OrderedFiles::from(i2.clone().into_iter().map(|v| (v, &[][..])));
        let ordered_files2_ = OrderedFiles::from(i2.clone().into_iter().map(|v| (v, &[][..])));
        let ordered_files = ordered_files1 + ordered_files2;
        for i in ordered_files1_.0 {
           assert!(ordered_files.0.iter().find(|f| f.name() == i.name()).is_some());
        }
        for i in ordered_files2_.0 {
           assert!(ordered_files.0.iter().find(|f| f.name() == i.name()).is_some());
        }
      }

      #[test]
      fn add_includes_iter(ref i1 in vec("\\PC*", 0..10), ref i2 in vec("\\PC*", 0..10)) {
        let ordered_files1 = OrderedFiles::from(i1.clone().into_iter().map(|v| (v, &[][..])));
        let ordered_files1_ = OrderedFiles::from(i1.clone().into_iter().map(|v| (v, &[][..])));
        let ordered_files2 = OrderedFiles::from(i2.clone().into_iter().map(|v| (v, &[][..])));
        let ordered_files2_ = OrderedFiles::from(i2.clone().into_iter().map(|v| (v, &[][..])));
        let ordered_files = ordered_files1 + ::std::iter::once(ordered_files2);
        for i in ordered_files1_.0 {
           assert!(ordered_files.0.iter().find(|f| f.name() == i.name()).is_some());
        }
        for i in ordered_files2_.0 {
           assert!(ordered_files.0.iter().find(|f| f.name() == i.name()).is_some());
        }
      }

     #[test]
     fn sub_excludes(ref names in vec("\\PC*", 0..10), i in 0..9) {
        prop_assume!(i as usize + 1 <= names.len());
        let ordered_files1 = OrderedFiles::from(names.clone().into_iter().map(|v| (v, &[][..])));
        let name = &names[i as usize];
        let ordered_files = ordered_files1 - name;
        assert!(ordered_files.0.iter().find(|f| f.name() == name).is_none());
      }

    }
}



/// Record is an immutable collection of files
pub trait Record {
   /// Implementation's type for reading files
   type Read : ::std::io::Read;
   /// Implementation's type for non-encoded hash
   type Hash : AsRef<[u8]>;
   /// Implementation's type for file names
   type Str : AsRef<str>;
   /// Implementation's iterator type for listing files
   type Iter : Iterator<Item=(Self::Str, Self::Read)>;
   /// Returns record hash
   fn hash(&self) -> Self::Hash;
   /// Returns encoded record hash
   ///
   /// The encoding is defined by its container (typically, the repository)
   /// and is intended to be human-readable and it MUST be an encoding of the
   /// byte array returned by [`hash`]
   ///
   /// [`hash`]: struct.Record.html#hash
   fn encoded_hash(&self) -> Self::Str;

   /// Returns enclosing item's ID
   fn item_id(&self) -> Self::Str;

   /// Returns an iterator over files in the record
   fn file_iter(&self) -> Self::Iter;
}


use serde_json::{Value as JsonValue, Map as JsonMap};
use serde::Serializer;
use serde::ser::SerializeStruct;

pub trait RecordExt: Record {

   fn has_type<S: AsRef<str>>(&self, typ: S) -> bool {
      let len = 6 + typ.as_ref().len();
      self.file_iter().any(|(name, _)| {
         let name = name.as_ref();
         name.len() == len &&
         name.starts_with(".type/") &&
         name.ends_with(typ.as_ref())
      })
   }

   fn file<S: AsRef<str>>(&self, file: S) -> Option<Self::Read> {
      let file = file.as_ref();
      self.file_iter().find(|&(ref name, _)| name.as_ref() == file).and_then(|(_, reader)| Some(reader))
   }

   fn serde_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where
        S: Serializer {
       use std::io::Read;
       let mut record = serializer.serialize_struct("Record", 2)?;
       let mut files = JsonMap::new();
       let mut buf = Vec::new();
       for (name, mut reader) in self.file_iter() {
           let name = name.as_ref().into();
           match reader.read_to_end(&mut buf) {
               Ok(_) => {
                   match ::std::str::from_utf8(&buf) {
                       Err(_) => {
                           let mut typ = JsonMap::new();
                           typ.insert("type".into(), JsonValue::String("binary".into()));
                           files.insert(name, JsonValue::Object(typ));
                       },
                       Ok(str) => {
                           files.insert(name, JsonValue::String(str.into()));
                       }
                   }
               },
               Err(err) => {
                   let mut error = JsonMap::new();
                   error.insert("error".into(), JsonValue::String(format!("{}", err)));
                   files.insert(name, JsonValue::Object(error));
               }
           }
           buf.clear();
       }
       record.serialize_field("hash", self.encoded_hash().as_ref().into())?;
       record.serialize_field("files", &JsonValue::Object(files))?;
       record.end()
    }

}

impl<T> RecordExt for T where T: Record {}