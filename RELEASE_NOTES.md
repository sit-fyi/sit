# 0.4.1

This is an interim release that should allow users of a released version to access
sit repositories with a flat records namespace enabled (currently a master-only feature)

# 0.4.0

This is primarily a bug fix release. It fixes a couple of important problems
in scanning record graphs and determinism of record hashes. It also
includes a few changes to improve performance in some cases.

# 0.3.0

With this release, SIT transitions from Serverless Issue Tracking to
Serverless Information Tracking.

This means that it is no longer limited to tracking issues. With
issue tracking functionality extracted to an independent module
(modules are one of the features introduced with this release),
you can track and share any kind of information.

The change that allowed this transition was mostly cosmetical
(*issue* got renamed to *item*) and a module convention was introduced
which makes transitioning pretty straightforward.

For existing issue tracking SIT repositories, all that needs to be
one is the addition of the `issue-tracking` module:

If you are using a Git repository and don't want to carry the entire contents of this repository, simply
using `git submodules` is a great way to accomplish this:

```
git submodule add https://github.com/sit-it/issue-tracking .sit/modules/issue-tracking
```

This allows to pin a specific version of `issue-tracking` and update it when neccessary.

When *not* using a Git repository, or if it is preferrable to carry all the modules within your SIT
repository (for example, if you don't want to depend on the availability of the module in the future),
you can simply copy the entire module into `.sit/modules/issue-tracking`:

```
git clone --depth=1 https://github.com/sit-it/issue-tracking .sit/modules/issue-tracking
rm -rf .sit/modules/issue-tracking/.sit .sit/modules/issue-tracking/.git
```

Other, more subtle changes are listed in the CHANGELOG.

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
