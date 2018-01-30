//! Record is an immutable collection of files

/// Record is an immutable collection of files
pub trait Record {
   /// Implementation's type for reading files
   type Read : ::std::io::Read;
   /// Implementation's type for file names
   type Str : AsRef<str>;
   /// Implementation's iterator type for listing files
   type Iter : Iterator<Item=(Self::Str, Self::Read)>;
   /// Returns record hash
   fn hash(&self) -> &[u8];
   /// Returns encoded record hash
   ///
   /// The encoding is defined by its container (typically, the repository)
   /// and is intended to be human-readable and it MUST be an encoding of the
   /// byte array returned by [`hash`]
   ///
   /// [`hash`]: struct.Record.html#hash
   fn encoded_hash(&self) -> Self::Str;
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