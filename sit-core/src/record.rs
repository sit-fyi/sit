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

}

impl<T> RecordExt for T where T: Record {}