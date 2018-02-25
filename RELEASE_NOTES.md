# 0.1.3

Maintenance release, made to make sure there's a release
version that can work with ongoing development of 0.2-compatible
repositories. `sit populate-files` must be run on pre-0.1.3 repositories.

# 0.1.2

Maintenance release. The core highlights are:

* Fixed a bug with handling some Unicode codepoints in certain
  scenarios.
* Improved performance.

# 0.1.1

Maintenance release. The core highlights are:

* Backport of a breaking change of the placement of a few directories inside
  of the SIT repository: `.web` becomes `web` and `.reducers` become `reducers`.
  Please rename them in your repositories accordingly.
* Adds `windows7` feature that enables building SIT for Windows 7
  (at this moment, not fully verified)

# 0.1.0

First public release. The intention is to get more people to try SIT out
to discover bugs, flaws and help generating more awareness for those who
are interested in this kind of tooling. It is by no means perfect, but
at some point we needed to cut a release!
