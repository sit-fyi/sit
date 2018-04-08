# 0.2.1

This is a maintenance release that allows users of a released version
to use SIT's own repository on master after a breaking change.

# 0.2.0

The first thing you'll probably notice about this release is the new front
page interface in sit-web. We've moved away from small tiles representing
issues to a more conventional list. This way title issue can always be
rendered in the given space. We also have a new logo designed by 
[Ura Design](https://ura.design). Thanks, guys!

This release also breaks a few things about how repositories and reducers
should be organized.

If you have a pre-0.1.1 SIT repository, make sure to
rename `.reducers` and `.web` directories inside of your reposutory to 
`reducers` and `web`, respectively.

Also, reducers must now use `module.export` to expose their function. You
should prepend your custom reducers with `module.export = `. For standard
reducers, if you haven't changed them, simply run `sit populate-files` inside
of your repository.

A more exciting addition to reducers is that now they can use `require()`
to load modules from inside of the `reducers` directory. All JavaScript files
directly under `reducers` directly will be loaded as reducers, and any
JavaScript files below that level can be loaded by those reducers using
`require()`. This will enable code re-use, use of third-party libraries
and other interesting featurs to come.

This release also addresses some of the performance issues found after
the release of 0.1.0.

SIT 0.2.0 also works on Windows 7 now (something 0.1.0 didn't have!)

# 0.1.0

First public release. The intention is to get more people to try SIT out
to discover bugs, flaws and help generating more awareness for those who
are interested in this kind of tooling. It is by no means perfect, but
at some point we needed to cut a release!
