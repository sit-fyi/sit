# MergeRequested

Indicates that the record includes a patchset to
be merged.

## Files

Since SIT is supposed to be SCM-agnostic (or even work with one), it supports
different ways to describe patchsets. Typically, only one way will be used
per record (typically, corresponding to the SCM currently used). In situations
when more than one method is used they all have to be *equivalent* patchsets.

### `git/*.patch`

Patches produced by `git format-patch`

### `patch/*.diff`

Diffs produced by `diff`

## State Effect

Appends `merge_requests` field (array) with the encoded record hash.
