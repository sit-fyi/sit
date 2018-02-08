function(state, record) {
    if (typeof record.files[".type/MergeRequested"] !== 'undefined') {
        var merge_requests = record.merge_requests || [];
        merge_requests.push(record.hash);
        return Object.assign({merge_requests: merge_requests}, state);
    }
    return state;
}