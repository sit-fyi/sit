## Modules

In order to facilitate use of SIT for diverse use cases, modularization feature
was developed. In short, it allows to define reducers, CLI, web interface and
other components externally.

The convention is very simple, you can put either a directory (or a symlink to one)
under .sit/modules and it will get automatically recognized. To support operating
systems without symlinks, instead of putting a directory, it can be a file with
a relative or absolute path to the module.

Currently used conventions for modules:

| Path                                | Description     |
|-------------------------------------|-----------------|
| .sit/module/MODULE/reducers/*.js    | Reducers        |
| .sit/module/MODULE/cli/sit-*[*.bat] | CLI subcommands |
| .sit/module/MODULE/web              | Web overlays    |
