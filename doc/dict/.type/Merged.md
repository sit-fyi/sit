# Merged

Signifies that a patch was merged

## Files

### `record`

Optional.

Contains an encoded record hash (the record that was the source of the
patch merged).

### `.authors`

Used to derive merge's authorship

### `.timestamp`

Used to derive merge's timestamp

## State Effect

Updates `comments` field with the an array containing
objects representing comments (three field: `text`, `authors`, `timestamp`)

Appends `merges` field with an object, containng `hash` referencing
`Merged` record and, optionally, `record` reference.