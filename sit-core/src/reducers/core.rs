//! Core reducers

use super::{Reducer, ChainedReducer};
use serde_json::{Map, Value as JsonValue};
use super::super::record::{Record, RecordExt};
use std::marker::PhantomData;
use std::io::Read;

/// Reduces SummaryChanged type
pub struct IssueSummaryReducer<R: Record>(PhantomData<R>);

impl<R: Record> IssueSummaryReducer<R> {
    pub fn new() -> Self {
        IssueSummaryReducer(PhantomData)
    }
}

impl<R: Record + RecordExt> Reducer for IssueSummaryReducer<R> {
    type State = Map<String, JsonValue>;
    type Item = R;

    fn reduce(&self, mut state: Self::State, item: &Self::Item) -> Self::State {
        if item.has_type("SummaryChanged") {
            state.insert("summary".into(), JsonValue::String(item.file("text")
                .and_then(|mut r| {
                    let mut s = String::new();
                    r.read_to_string(&mut s).unwrap();
                    Some(String::from(s.trim()))
                })
                .or_else(|| Some(String::from("")))
                .unwrap()));
            state
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
    type State = Map<String, JsonValue>;
    type Item = R;

    fn reduce(&self, mut state: Self::State, item: &Self::Item) -> Self::State {
        if item.has_type("DetailsChanged") {
            state.insert("details".into(), JsonValue::String(item.file("text")
                .and_then(|mut r| {
                    let mut s = String::new();
                    r.read_to_string(&mut s).unwrap();
                    Some(s)
                })
                .or_else(|| Some(String::from("")))
                .unwrap()));
            state
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
    type State = Map<String, JsonValue>;
    type Item = R;

    fn reduce(&self, mut state: Self::State, item: &Self::Item) -> Self::State {
        state.entry("state").or_insert(JsonValue::String("open".into()));
        if item.has_type("Closed") {
            state.insert("state".into(), JsonValue::String("closed".into()));
            state
        } else if item.has_type("Reopened") {
            state.insert("state".into(), JsonValue::String("open".into()));
            state
        } else {
            state
        }
    }
}

/// Reduces Commented type
pub struct CommentedReducer<R: Record>(PhantomData<R>);

impl<R: Record> CommentedReducer<R> {
    pub fn new() -> Self {
        CommentedReducer(PhantomData)
    }
}

impl<R: Record + RecordExt> Reducer for CommentedReducer<R> {
    type State = Map<String, JsonValue>;
    type Item = R;

    fn reduce(&self, mut state: Self::State, item: &Self::Item) -> Self::State {
        state.entry("comments").or_insert(JsonValue::Array(vec![]));
        if item.has_type("Commented") {
            // scope it to unborrow `map` before it is returned
            {
                let comments = state.get_mut("comments").unwrap();
                let mut comment: Map<String, JsonValue> = Default::default();
                comment.insert("text".into(), JsonValue::String(item.file("text")
                    .and_then(|mut r| {
                        let mut s = String::new();
                        r.read_to_string(&mut s).unwrap();
                        Some(s)
                    })
                    .or_else(|| Some(String::from("")))
                    .unwrap()));
                comment.insert("authors".into(), JsonValue::String(item.file(".authors")
                    .and_then(|mut r| {
                        let mut s = String::new();
                        r.read_to_string(&mut s).unwrap();
                        Some(s)
                    })
                    .or_else(|| Some(String::from("")))
                    .unwrap()));
                comment.insert("timestamp".into(), JsonValue::String(item.file(".timestamp")
                    .and_then(|mut r| {
                        let mut s = String::new();
                        r.read_to_string(&mut s).unwrap();
                        Some(s)
                    })
                    .or_else(|| Some(String::from("")))
                    .unwrap()));
                comments.as_array_mut().unwrap().push(JsonValue::Object(comment));
            }
            state
        } else {
            state
        }
    }
}

/// Reduces Commented type
pub struct MergeRequestedReducer<R: Record>(PhantomData<R>);

impl<R: Record> MergeRequestedReducer<R> {
    pub fn new() -> Self {
        MergeRequestedReducer(PhantomData)
    }
}

impl<R: Record + RecordExt> Reducer for MergeRequestedReducer<R> {
    type State = Map<String, JsonValue>;
    type Item = R;

    fn reduce(&self, mut state: Self::State, item: &Self::Item) -> Self::State {
        state.entry("merge_requests").or_insert(JsonValue::Array(vec![]));
        if item.has_type("MergeRequested") {
            // scope it to unborrow `requests` before it is returned
            {
                let requests = state.get_mut("merge_requests").unwrap();
                let hash = item.encoded_hash();
                requests.as_array_mut().unwrap().push(JsonValue::String(hash.as_ref().into()));
            }
            state
        } else {
            state
        }
    }
}


/// Combines Closed, SummaryChanged, DetailsChanged, Commented, MergeRequested reducers
pub struct BasicIssueReducer<R: Record>(ChainedReducer<MergeRequestedReducer<R>,
    ChainedReducer<CommentedReducer<R>, ChainedReducer< ChainedReducer<IssueClosureReducer<R>, IssueSummaryReducer<R>>,
    IssueDetailsReducer<R>>>>);


impl<R: Record> BasicIssueReducer<R> {
    pub fn new() -> Self {
       BasicIssueReducer(MergeRequestedReducer::new().chain(CommentedReducer::new()
                             .chain(IssueClosureReducer::new()
                             .chain(IssueSummaryReducer::new())
                             .chain(IssueDetailsReducer::new()))))
    }
}

impl<R: Record> Reducer for BasicIssueReducer<R> {
    type State = Map<String, JsonValue>;
    type Item = R;

    fn reduce(&self, state: Self::State, item: &Self::Item) -> Self::State {
        self.0.reduce(state, item)
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::*;

    use issue::{Issue, IssueReduction};
    use Repository;

    #[test]
    fn summary() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        let issue = repo.new_issue().unwrap();
        // no SummaryChanged
        let state = issue.reduce_with_reducer(IssueSummaryReducer::new()).unwrap();
        assert!(!state.contains_key("summary"));
        // one SummaryChanged
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"Title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(IssueSummaryReducer::new()).unwrap();
        assert_eq!(state.get("summary").unwrap().as_str().unwrap(), "Title");
        // two SummaryChanged items
        issue.new_record(vec![(".type/SummaryChanged", &b""[..]), ("text", &b"New title"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(IssueSummaryReducer::new()).unwrap();
        assert_eq!(state.get("summary").unwrap().as_str().unwrap(), "New title");
    }

    #[test]
    fn details() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        let issue = repo.new_issue().unwrap();
        // no DetailsChanged
        let state = issue.reduce_with_reducer(IssueDetailsReducer::new()).unwrap();
        assert!(!state.contains_key("details"));
        // one DetailsChanged
        issue.new_record(vec![(".type/DetailsChanged", &b""[..]), ("text", &b"Explanation"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(IssueDetailsReducer::new()).unwrap();
        assert_eq!(state.get("details").unwrap().as_str().unwrap(), "Explanation");
        // two DetailsChanged items
        issue.new_record(vec![(".type/DetailsChanged", &b""[..]), ("text", &b"New explanation"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(IssueDetailsReducer::new()).unwrap();
        assert_eq!(state.get("details").unwrap().as_str().unwrap(), "New explanation");
    }

    #[test]
    fn closure() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        let issue = repo.new_issue().unwrap();
        // Closed
        issue.new_record(vec![(".type/Closed", &b""[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(IssueClosureReducer::new()).unwrap();
        assert_eq!(state.get("state").unwrap().as_str().unwrap(), "closed");
        // Reopened
        issue.new_record(vec![(".type/Reopened", &b""[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(IssueClosureReducer::new()).unwrap();
        assert_eq!(state.get("state").unwrap().as_str().unwrap(), "open");
    }

    #[test]
    fn commented() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        let issue = repo.new_issue().unwrap();
        issue.new_record(vec![(".type/Commented", &b""[..]), ("text", &b"Comment 1"[..]),
                              (".timestamp", &b"2018-01-30T16:24:59.385560008Z"[..]),
                              (".authors", &b"John Doe <john@foobar.com>"[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(CommentedReducer::new()).unwrap();
        let comments = state.get("comments").unwrap().as_array().unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].get("text").unwrap().as_str().unwrap(), "Comment 1");
        assert_eq!(comments[0].get("authors").unwrap().as_str().unwrap(), "John Doe <john@foobar.com>");
        assert_eq!(comments[0].get("timestamp").unwrap().as_str().unwrap(), "2018-01-30T16:24:59.385560008Z");

    }

    #[test]
    fn merge_requested() {
        let mut tmp = TempDir::new("sit").unwrap().into_path();
        tmp.push(".sit");
        let repo = Repository::new(tmp).unwrap();
        let issue = repo.new_issue().unwrap();
        let record = issue.new_record(vec![(".type/MergeRequested", &b""[..])].into_iter(), true).unwrap();
        let state = issue.reduce_with_reducer(MergeRequestedReducer::new()).unwrap();
        let requests = state.get("merge_requests").unwrap().as_array().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0], record.encoded_hash());
    }


}