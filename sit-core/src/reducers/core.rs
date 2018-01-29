//! Core reducers

use super::{Reducer, ChainedReducer};
use serde_json::Value as JsonValue;
use super::super::record::{Record, RecordExt};
use std::marker::PhantomData;

/// Reduces SummaryChanged type
pub struct IssueSummaryReducer<R: Record>(PhantomData<R>);

impl<R: Record> IssueSummaryReducer<R> {
    pub fn new() -> Self {
        IssueSummaryReducer(PhantomData)
    }
}

impl<R: Record + RecordExt> Reducer for IssueSummaryReducer<R> {
    type State = JsonValue;
    type Item = R;

    fn reduce(&self, state: Self::State, item: &Self::Item) -> Self::State {
        use std::io::Read;;
        if item.has_type("SummaryChanged") {
            match state {
                JsonValue::Object(mut map) => {
                    map.insert("summary".into(), JsonValue::String(item.file("text")
                        .and_then(|mut r| {
                            let mut s = String::new();
                            r.read_to_string(&mut s).unwrap();
                            Some(String::from(s.trim()))
                        })
                        .or_else(|| Some(String::from("")))
                        .unwrap()));
                    JsonValue::Object(map)
                }
                _ => panic!("invalid state"),
            }
        } else {
            state
        }
    }
}


/// Reduces DetailsChanged type
pub struct IssueDetailsReducer<R: Record>(PhantomData<R>);

impl<R: Record> IssueDetailsReducer<R> {
    pub fn new() -> Self {
        IssueDetailsReducer(PhantomData)
    }
}

impl<R: Record + RecordExt> Reducer for IssueDetailsReducer<R> {
    type State = JsonValue;
    type Item = R;

    fn reduce(&self, state: Self::State, item: &Self::Item) -> Self::State {
        use std::io::Read;;
        if item.has_type("DetailsChanged") {
            match state {
                JsonValue::Object(mut map) => {
                    map.insert("details".into(), JsonValue::String(item.file("text")
                        .and_then(|mut r| {
                            let mut s = String::new();
                            r.read_to_string(&mut s).unwrap();
                            Some(s)
                        })
                        .or_else(|| Some(String::from("")))
                        .unwrap()));
                    JsonValue::Object(map)
                }
                _ => panic!("invalid state"),
            }
        } else {
            state
        }
    }
}

/// Reduces Closed type
pub struct IssueClosureReducer<R: Record>(PhantomData<R>);

impl<R: Record> IssueClosureReducer<R> {
    pub fn new() -> Self {
        IssueClosureReducer(PhantomData)
    }
}

impl<R: Record + RecordExt> Reducer for IssueClosureReducer<R> {
    type State = JsonValue;
    type Item = R;

    fn reduce(&self, state: Self::State, item: &Self::Item) -> Self::State {
        let state = match state {
            JsonValue::Object(mut map) => {
                map.entry("state").or_insert(JsonValue::String("open".into()));
                JsonValue::Object(map)
            },
            _ => panic!("invalid state"),
        };
        if item.has_type("Closed") {
            match state {
                JsonValue::Object(mut map) => {
                    map.insert("state".into(), JsonValue::String("closed".into()));
                    JsonValue::Object(map)
                }
                _ => panic!("invalid state"),
            }
        } else if item.has_type("Reopened") {
             match state {
                JsonValue::Object(mut map) => {
                    map.insert("state".into(), JsonValue::String("open".into()));
                    JsonValue::Object(map)
                }
                _ => panic!("invalid state"),
            }
        } else {
            state
        }
    }
}


/// Combines Closed, SummaryChanged, DetailsChanged reducers
pub struct BasicIssueReducer<R: Record>(ChainedReducer<ChainedReducer<IssueClosureReducer<R>, IssueSummaryReducer<R>>, IssueDetailsReducer<R>>);

impl<R: Record> BasicIssueReducer<R> {
    pub fn new() -> Self {
       BasicIssueReducer(IssueClosureReducer::new().chain(IssueSummaryReducer::new()).chain(IssueDetailsReducer::new()))
    }
}

use std::ops::Deref;

impl<R: Record> Deref for BasicIssueReducer<R> {
    type Target = ChainedReducer<ChainedReducer<IssueClosureReducer<R>, IssueSummaryReducer<R>>, IssueDetailsReducer<R>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::*;

    use issue::Issue;
    use Repository;

    fn reduce<Records: IntoIterator<Item=I::Record> + AsRef<[I::Record]>, I: Issue<Records=Records>,
              R: Reducer<State = JsonValue, Item = I::Record>>(issue: &I, reducer: &R) -> JsonValue {
        match issue.record_iter() {
            Ok(records) => records
                .fold(JsonValue::Object(Default::default()), |state, items|
                    items.into_iter().fold(state, |state, item|
                        reducer.reduce(state, &item))),
            _ => panic!("can't iterate over records"),
        }
    }

    #[test]
    fn summary() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        let issue = repo.new_issue().unwrap();
        // no SummaryChanged
        let state = reduce(&issue, &IssueSummaryReducer::new());
        assert!(!state.as_object().unwrap().contains_key("summary"));
        // one SummaryChanged
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = reduce(&issue, &IssueSummaryReducer::new());
        assert_eq!(state.as_object().unwrap().get("summary").unwrap().as_str().unwrap(), "Title");
        // two SummaryChanged items
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"New title"[..])].into_iter(), true).unwrap();
                let state = reduce(&issue, &IssueSummaryReducer::new());
        assert_eq!(state.as_object().unwrap().get("summary").unwrap().as_str().unwrap(), "New title");
    }

    #[test]
    fn details() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        let issue = repo.new_issue().unwrap();
        // no DetailsChanged
        let state = reduce(&issue, &IssueDetailsReducer::new());
        assert!(!state.as_object().unwrap().contains_key("details"));
        // one DetailsChanged
        issue.new_record(vec![(".type/DetailsChanged", &b""[..]), ("text", &b"Explanation"[..])].into_iter(), true).unwrap();
        let state = reduce(&issue, &IssueDetailsReducer::new());
        assert_eq!(state.as_object().unwrap().get("details").unwrap().as_str().unwrap(), "Explanation");
        // two DetailsChanged items
        issue.new_record(vec![(".type/DetailsChanged", &b""[..]), ("text", &b"New explanation"[..])].into_iter(), true).unwrap();
        let state = reduce(&issue, &IssueDetailsReducer::new());
        assert_eq!(state.as_object().unwrap().get("details").unwrap().as_str().unwrap(), "New explanation");
    }

    #[test]
    fn closure() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        let issue = repo.new_issue().unwrap();
        // Closed
        issue.new_record(vec![(".type/Closed", &b""[..])].into_iter(), true).unwrap();
        let state = reduce(&issue, &IssueClosureReducer::new());
        assert_eq!(state.as_object().unwrap().get("state").unwrap().as_str().unwrap(), "closed");
        // Reopened
        issue.new_record(vec![(".type/Reopened", &b""[..])].into_iter(), true).unwrap();
        let state = reduce(&issue, &IssueClosureReducer::new());
        assert_eq!(state.as_object().unwrap().get("state").unwrap().as_str().unwrap(), "open");
    }

}