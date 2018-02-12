function(state, record) {
    if (typeof record.files[".type/DetailsChanged"] !== 'undefined') {
        var merge_request = !!record.files[".type/MergeRequested"] ? record.hash : null ;
        return Object.assign(state, {merge_request: merge_request, details: new TextDecoder("utf-8").decode(record.files.text).trim()});
    } else {
        return state;
    }
}
