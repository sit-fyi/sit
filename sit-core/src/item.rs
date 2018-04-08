//! Every repository acts as a container for items

use serde_json::{Map, Value};

use super::Reducer;

#[derive(Debug, Error)]
pub enum ReductionError<Err: ::std::error::Error + ::std::fmt::Debug> {
    ImplementationError(Err)
}

/// Because of SIT's extensible nature, item can
/// be used to represent a wild variety of entities, such
/// as issue, documents, accounts, etc.
pub trait Item: Sized {
    /// Error type used by the implementation
    type Error: ::std::error::Error + ::std::fmt::Debug;
    /// Record type used by the implementation
    type Record : super::Record;
    /// Type used to list records that can be referenced as a slice of records
    type Records : IntoIterator<Item=Self::Record>;
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

/// [`Issue`] trait extension that defines and implements default reduction algorithms
///
/// [`Issue`]: trait.Issue.html
pub trait ItemReduction: Item {

    /// Reduces item with a given [`Reducer`]
    ///
    /// Will insert item's `id` into the initial state
    ///
    /// [`Reducer`]: ../reducers/trait.Reducer.html
    fn reduce_with_reducer<R: Reducer<State=Map<String, Value>, Item=Self::Record>>(&self, reducer: &mut R) -> Result<Map<String, Value>, ReductionError<Self::Error>> {
        let records = self.record_iter()?;
        let mut state: Map<String, Value> = Default::default();
        state.insert("id".into(), Value::String(self.id().into()));
        Ok(records.fold(state, |acc, recs|
            recs.into_iter().fold(acc, |acc, rec| reducer.reduce(acc, &rec))))
    }


}

impl<T> ItemReduction for T where T: Item {}

