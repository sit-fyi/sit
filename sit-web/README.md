# sit-web

## Development process

Currently, the easiest way to develop updates for `webapp` is
to copy all files into `.sit/.web` and work on them, since they
will be automatically reloaded. Once done, remove everything
from `webapp`, copy `.sit/.web`'s content over and re-add it to
git. Yes, that includes `bower_components`.

