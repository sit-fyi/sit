function(state, record) {
    if (typeof this.state == 'undefined') {
        this.state = 'open';
    }
    if (typeof record.files[".type/Closed"] !== 'undefined') {
        this.state = 'closed';
        return Object.assign({state: 'closed'}, state);
    }
    if (typeof record.files[".type/Reopened"] !== 'undefined') {
        this.state = 'open';
        return Object.assign({state: 'open'}, state);
    }
    return Object.assign({state: this.state}, state);
}