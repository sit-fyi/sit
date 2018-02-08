function(state, record) {
    if (typeof record.files[".type/DetailsChanged"] !== 'undefined') {
        return Object.assign(state, {details: new TextDecoder("utf-8").decode(record.files.text).trim()});
    } else {
        return state;
    }
}
