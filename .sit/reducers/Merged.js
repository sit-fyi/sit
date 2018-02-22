module.exports = function(state, record) {

    if (typeof record.files[".type/Merged"] !== 'undefined') {
        var decoder = new TextDecoder("utf-8");
        var merges = this.merges || [];

        var merge_request = state.merge_request || "";

        if (typeof record.files["record"] !== 'undefined') {
            merge_request = decoder.decode(record.files["record"]);
        }

        merges.push({hash: record.hash, record: merge_request});

        this.merges = merges;

        return Object.assign(state, {merges: merges});
    }
    return state;
}