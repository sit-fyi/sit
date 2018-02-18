function(state, record) {
    if (typeof record.files[".type/Commented"] !== 'undefined') {
        var comments = this.comments || [];
        var decoder = new TextDecoder("utf-8");
        var merge_request = !!record.files[".type/MergeRequested"] ? record.hash : null;
        comments.push({
            text: decoder.decode(record.files.text),
            authors: decoder.decode(record.files[".authors"]),
            timestamp: decoder.decode(record.files[".timestamp"]),
            merge_request: merge_request,
        });
        this.comments = comments;
        return Object.assign(state, {comments: comments});
    }
    return state;
}