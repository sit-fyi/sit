function(state, record) {
    if (typeof record.files[".type/SummaryChanged"] !== 'undefined') {
        return Object.assign({summary: new TextDecoder("utf-8").decode(record.files.text).trim()}, state);
    } else {
        return state;
    }
}