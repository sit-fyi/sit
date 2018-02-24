module.exports = function(state, record) {
    if (typeof record.files[".timestamp"] !== 'undefined') {
        return Object.assign(state, {last_updated_timestamp: new TextDecoder('utf-8').decode(record.files[".timestamp"])})
    }
}