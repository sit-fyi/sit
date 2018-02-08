function(state, record) {
    if (typeof record.files[".type/SummaryChanged"] !== 'undefined') {
        return Object.assign(state, {summary: new TextDecoder("utf-8").decode(record.files.text).trim()});
    } else {
        return state;
    }
}