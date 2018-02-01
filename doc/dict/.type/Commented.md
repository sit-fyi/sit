# Commented

Adds a comment to an issue

## Files

### `text`

Required.

Contains A UTF-8 string with the comment

### `.authors`

Used to derive comment's authorship

### `.timestamp`

Used to derive comment's timestamp

## State Effect

Updates `comments` field with the an array containing
objects representing comments (three field: `text`, `authors`, `timestamp`)