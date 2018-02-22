module.exports = function(state, record) {
    if (typeof record.files[".type/MergeRequested"] !== 'undefined') {
        var merge_requests = record.merge_requests || [];
        merge_requests.push(record.hash);
        return Object.assign(state, {merge_requests: merge_requests});
    }
    return state;
}