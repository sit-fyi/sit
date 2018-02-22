module.exports = function(state, record) {
    var state = state;

    // Handle Merged as a comment, too
    if (typeof record.files[".type/Merged"] !== 'undefined') {
        var comments = this.comments || [];
        var decoder = new TextDecoder("utf-8");
        var merge_request = state.merge_request || "";
        if (typeof record.files["record"] !== 'undefined') {
            merge_request = decoder.decode(record.files["record"]);
        }
        comments.push({
            text: ("Merged " + merge_request).trim(),
            authors: decoder.decode(record.files[".authors"]),
            timestamp: decoder.decode(record.files[".timestamp"]),
        });
        this.comments = comments;
        state = Object.assign(state, {comments: comments});
    }

    if (typeof record.files[".type/Commented"] !== 'undefined') {
        var comments = this.comments || [];
        var decoder = new TextDecoder("utf-8");
        var merge_request = !!record.files[".type/MergeRequested"] ? record.hash : null;
        var merge_request_report = !!record.files[".type/MergeRequestVerificationSucceeded"] ?
            "success" : null;
        merge_request_report = merge_request_report || !!record.files[".type/MergeRequestVerificationFailed"] ?
            "failure" : null;
        comments.push({
            text: decoder.decode(record.files.text),
            authors: decoder.decode(record.files[".authors"]),
            timestamp: decoder.decode(record.files[".timestamp"]),
            merge_request: merge_request,
            merge_request_report: merge_request_report,
        });
        this.comments = comments;
        state = Object.assign(state, {comments: comments});
    }

    return state;
}