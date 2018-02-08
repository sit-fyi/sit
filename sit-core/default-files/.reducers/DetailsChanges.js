function(state, record) {
    if (typeof record.files[".type/DetailsChanged"] !== 'undefined') {
        return Object.assign({details: new TextDecoder("utf-8").decode(record.files.text).trim()}, state);
    } else {
        return state;
    }
}
