//! Every repository consists of issues

/// Issue is a topic or a problem for debate, discussion
/// and resolution. Also known as a "ticket".
///
/// Because of SIT's extensible nature, issue can be also
/// be used to represent a wild variety of entities. For example,
/// a Kanban board with its records representing movement of other
/// issues into, across and out of the board.
pub trait Issue {
    /// Error type used by the implementation
    type Error;
    /// Record type used by the implementation
    type Record : super::Record;
    /// Type used to list records that can be referenced as a slice of records
    type Records : AsRef<[Self::Record]>;
    /// Iterator over lists of records
    type RecordIter : Iterator<Item=Self::Records>;
    /// Issue must have an ID, ideally human-readable
    fn id(&self) -> &str;
    /// Iterates through the tree of records
    fn record_iter(&self) -> Result<Self::RecordIter, Self::Error>;
    /// Creates and returns a new record.
    ///
    /// Will reference all dangling records as its parent, unless
    /// `link_parents` is set to `false`
    fn new_record<S: AsRef<str>, R: ::std::io::Read,
                  I: Iterator<Item=(S, R)>>(&self, iter: I, link_parents: bool)
       -> Result<Self::Record, Self::Error>;
}