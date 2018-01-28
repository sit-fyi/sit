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