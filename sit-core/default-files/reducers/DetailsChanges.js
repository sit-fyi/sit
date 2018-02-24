module.exports = function(state, record) {
    if (typeof record.files[".type/DetailsChanged"] !== 'undefined') {
        var merge_request = !!record.files[".type/MergeRequested"] ? record.hash : null ;
        var decoder = new TextDecoder("utf-8");
        return Object.assign(state, {
            authors: state.authors || decoder.decode(record.files[".authors"]),
            merge_request: merge_request,
            details: decoder.decode(record.files.text).trim(),
            timestamp: state.timestamp || decoder.decode(record.files[".timestamp"]),
        });
    } else {
        return state;
    }
}
