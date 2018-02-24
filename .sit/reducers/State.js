module.exports = function(state, record) {
    if (typeof this.state == 'undefined') {
        this.state = 'open';
    }
    if (typeof record.files[".type/Closed"] !== 'undefined') {
        this.state = 'closed';
        return Object.assign(state, {state: 'closed'});
    }
    if (typeof record.files[".type/Reopened"] !== 'undefined') {
        this.state = 'open';
        return Object.assign(state, {state: 'open'});
    }
    return Object.assign(state, {state: this.state});
}