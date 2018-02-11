function(state, record) {
    if (typeof record.files[".type/Commented"] !== 'undefined') {
        var comments = this.comments || [];
        var decoder = new TextDecoder("utf-8");
        comments.push({
            text: decoder.decode(record.files.text),
            authors: decoder.decode(record.files[".authors"]),
            timestamp: decoder.decode(record.files[".timestamp"])
        });
        this.comments = comments;
        return Object.assign(state, {comments: comments});
    }
    return state;
}