//! Every repository acts as a container for items

use serde_json::{Map, Value};

use record::{RecordOwningContainer, RecordContainerReduction};

/// Because of SIT's extensible nature, item can
/// be used to represent a wild variety of entities, such
/// as issue, documents, accounts, etc.
pub trait Item: RecordOwningContainer {
    /// Item must have an ID, ideally human-readable
    fn id(&self) -> &str;
}


impl<T> RecordContainerReduction for T where T: Item {
    fn initialize_state(&self, mut state: Map<String, Value>) -> Map<String, Value> {
        state.insert("id".into(), Value::String(self.id().into()));
        state
    }
}

