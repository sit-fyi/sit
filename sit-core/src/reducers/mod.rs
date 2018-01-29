//! Reducers process issues' records to present a digestable view
//!

/// Generic reducer trait
pub trait Reducer: Sized {
    /// State type
    type State;
    /// Item type
    type Item;

    /// Takes current state, item and returns new state
    fn reduce(&self, state: Self::State, item: &Self::Item) -> Self::State;
    /// Chains two reducers together sequentially
    fn chain<R: Reducer<State=Self::State, Item=Self::Item>>(self, other: R) -> ChainedReducer<Self, R> {
       ChainedReducer::new(self, other)
    }
}

pub mod core;
pub use self::core::BasicIssueReducer;

/// Chained reducer (consists of two reducers)
///
/// Will apply first and then second reducer to a given state
/// when used as a reducer itself
pub struct ChainedReducer<R1: Reducer, R2: Reducer>(R1, R2);

impl<R1: Reducer, R2: Reducer> ChainedReducer<R1, R2> {
    /// Returns a new chain reducer
    pub fn new(r1: R1, r2: R2) -> Self {
        ChainedReducer(r1, r2)
    }
}


impl<T, I, R1: Reducer<State=T, Item=I>, R2: Reducer<State=T, Item=I>> Reducer for ChainedReducer<R1, R2> {
    type State = R1::State;
    type Item = R1::Item;

    fn reduce(&self, state: Self::State, item: &Self::Item) -> Self::State {
        self.1.reduce(self.0.reduce(state, item), item)
    }
}

#[cfg(test)]
mod tests {

    use super::Reducer;

    struct R<T>(T);

    impl<T: Clone> Reducer for R<T> {
        type State = T;
        type Item = T;

        fn reduce(&self, _state: Self::State, _item: &Self::Item) -> Self::State {
            self.0.clone()
        }
    }

    #[test]
    fn chained_reducer() {
        assert_eq!(R(1).chain(R(2)).reduce(0, &0), 2);
    }

}