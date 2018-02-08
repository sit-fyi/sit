function(state, record) {
    if (typeof record.files[".type/Commented"] !== 'undefined') {
        var comments = record.comments || [];
        var decoder = new TextDecoder("utf-8");
        comments.push({
            text: decoder.decode(record.files.text),
            authors: decoder.decode(record.files[".authors"]),
            timestamp: decoder.decode(record.files[".timestamp"])
        });
        return Object.assign({comments: comments}, state);
    }
    return state;
}