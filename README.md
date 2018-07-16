<h1 align="center">
  <br>
  <a href="http://sit.fyi"><img src="logo.png" alt="SIT" width="150"></a>
  <br>
  <br>
  SIT
  <br>
</h1>

<h4 align="center">Serverless Information Tracker</h4>

<p align="center">
 <a href="https://github.com/sit-fyi/sit/releases"><img alt="Release" src="https://img.shields.io/github/release/sit-fyi/sit.svg"></a>
 <a href="https://gitter.im/sit-fyi/Lobby"><img alt="Chat" src="https://badges.gitter.im/sit-fyi/Lobby.png"></a>
 <a href="https://travis-ci.org/sit-fyi/sit"><img alt="Build status" src="https://travis-ci.org/sit-fyi/sit.svg?branch=master"></a>
 <a href="https://ci.appveyor.com/project/yrashk/sit"><img alt="Windows Build status" src="https://ci.appveyor.com/api/projects/status/0iv6ltgk3pa122hx?svg=true"></a>
 <img alt="issues open/total" src="https://s3-us-west-1.amazonaws.com/sit-badges/issues.svg?refresh">
 <img alt="merge requests open/total" src="https://s3-us-west-1.amazonaws.com/sit-badges/merge_requests.svg?refresh">
 <img alt=Backers on Open Collective" src="https://opencollective.com/sit/backers/badge.svg">
 <img alt="Sponsors on Open Collective" src="https://opencollective.com/sit/sponsors/badge.svg">
</p>


 
<p align="center">
  [
    <a href="https://github.com/sit-fyi/sit/releases"><b>Download</b></a> |
    <a href="doc/architecture_overview.md"><b>Overview</b></a> |
    <a href="#questions-bug-reports-etc"><b>Issues & Merge Requests</b></a>
  ]
</p>

SIT is a compact tool that helps tracking and sharing information between people and systems
in a decentralized, sporadically-online environment (*aka "the real world"*).

Its goal is to lower the barrier for recording, querying and sharing information
independently. Instead of having to setup and maintain a server and a database,
or having to rely on services of an external third party, SIT is a self-contained
binary for Linux, OS X and Windows that typically works on the end-user's computer.
SIT's medium of record is files. No external database is required.

## Modules

While bare SIT can track any kind of information (*it's all just files, after
all*), the user experience of using it as is might be less than exciting. For this
reason, SIT supports a concept of modules that allows to operate domain-specific
workflows and interfaces easily.

Currently available modules:

* [Issue Tracking](https://github.com/sit-fyi/issue-tracking)

## Why Should I Care?

As far as analogies go, we're doing to information tracking what Git did to version control systems. But let us
further elaborate on a few benefits to consider:

* **Works offline**. You can synchronize information, go offline and work
  on it without needing a connection. You can synchronize at any time later.
* **Contextualizes state**. When used together with an SCM (such as Git), you
  can see the state of any item at any given revision (in the context of issue
  tracking, for example, it can answer the question of *"what release branches
  is this issue closed on?"*)
* **Continuously localizes data**. You can access the data at any time. No API rate limits. It's on your filesystem.
* **Adapts to your group topology**. Synchronization can be done over Git, Dropbox, Keybase,
  USB flash drives or anything else that allows you to copy files between computers.
* **Malleable**. You can make it handle just about any workflow and payload. The customization
  is in its blood.

## Project Status

It is in the early adopter stage. It's usable but not everything is done yet and
some things will change. We're publishing releases regularly but always encourage
trying out the latest and greatest master branch.

Originally IT in SIT stood for "issue tracking". Since then, it grew to become a generalized
information tracking tool (with issue tracking extracted to a module)

## Obtaining SIT

All our releases are hosted on [GitHub](https://github.com/sit-fyi/sit/releases)
and binary files can be downloaded from there.

You can also use this oneliner to install it for your local user:

```
curl -s https://sit.fyi/install.sh | sh
```

*Please note that while this is a convenient way to install SIT, it is not
the most secure one because you're trusting install.sh to not do any harm.
We're doing our best (within reason) to ensure this file isn't hijacked by a malicious
actor. If this is a concern for you, please use the links referenced above or
build SIT from sources.*



## Build Instructions

As SIT is currently in its early days, sometimes it might make sense to use a
pre-release build. We encourage that. It helps us building a better product.

Firstly, you will need to install Rust 1.26 and CMake. Luckily
it is typically a very simple process. You can find
instructions on [Rust's website](https://www.rust-lang.org/en-US/install.html).

Now, after that has been taken care of, time to check
out SIT and build it:

```
git clone https://github.com/sit-fyi/sit
cd sit
cargo build --release
```

Now, you can copy `target/release/sit` and `target/release/sit-web` to your
`PATH` or add `/path/to/target/release` to `PATH` to always have the most
recent version available.

## Questions, Bug Reports, etc.?

SIT's is using SIT for tracking issues (duh!) and because of this, GitHub
issues are turned off. It's a good excuse to try out SIT if you have an
issue to file!

You will get all issue updates when you fetch this git repository. All updates
will come through it as well.

Simply run `sit-web` in this repository's clone and open it in the browser.

#### Send Updates to Upstream

Once you've used sit-web or `sit mr` to work on the issues,
you can send the updates to this repository:

1. Create a branch (as a convention, you can use your issue ID or an added record ID as a branch name, but free to choose anything else, preferrably unique)
2. Add new files in `.sit` and commit them. Commit message can be simply "Added issue <ISSUE-ID>"
   or, say, "Commented on issue <ISSUE-ID>"
3. Push it out to the Inbox: `GIT_SSH_COMMAND="ssh -i sit-inbox" git push git@git.sit.fyi:sit-fyi/sit-inbox.git <branch>`
4. If the commit only contains new records (nothing else is permitted!) the Inbox
   will accept the push and immediately forward it to sit's master repository on GitHub.
   Otherwise, the push will be rejected.

To further simplify the process of sending records to the upstream,
it's highly recommended to add a remote (such as `issues`) for `git@git.sit.fyi:sit-fyi/sit-inbox.git`
and this to your `~/.ssh/config`:

```
host git.sit.fyi
  HostName git.sit.fyi
  IdentityFile /path/to/sit/repo/sit-inbox
  User git
```

This way, pushing out, will be as nice as `git push issues <branch>`

### Preparing a merge request

Please refer to [CONTRIBUTING](https://github.com/sit-fyi/sit/blob/master/CONTRIBUTING.md#preparing-a-merge-request) for the instruction.




## License

SIT is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See LICENSE-APACHE and LICENSE-MIT for details.

## Credits

Shout-out to [Ura Design](https://ura.design/) for designing the previous version of our logo, hope we wore it well! They help with [design magic](https://ura.design/request/) for open source projects.

## Contributing

This project is in its very early days and we will always be welcoming
contributors.

Our goal is to encourage frictionless contributions to the project. In order to
achieve that, we use Unprotocols' [C4 process](https://rfc.unprotocols.org/spec:1/C4)
as an inspiration. Please read it, it will answer a lot of questions. Our goal is to
merge patches as quickly as possible and make new stable releases regularly.

In a nutshell, this means:

* We merge patches rapidly (try!)
* We are open to diverse ideas
* We prefer code now over consensus later

To learn more, read our [contribution guidelines](CONTRIBUTING.md)

### Contributors

This project exists thanks to all the people who contribute. [[Contribute](CONTRIBUTING.md)].
<a href="graphs/contributors"><img src="https://opencollective.com/sit/contributors.svg?width=890&button=false" /></a>


### Backers

Thank you to all our backers! 🙏 [[Become a backer](https://opencollective.com/sit#backer)]

<a href="https://opencollective.com/sit#backers" target="_blank"><img src="https://opencollective.com/sit/backers.svg?width=890"></a>


### Sponsors

Support this project by becoming a sponsor. Your logo will show up here with a link to your website. [[Become a sponsor](https://opencollective.com/sit#sponsor)]

<a href="https://opencollective.com/sit/sponsor/0/website" target="_blank"><img src="https://opencollective.com/sit/sponsor/0/avatar.svg"></a>
<a href="https://opencollective.com/sit/sponsor/1/website" target="_blank"><img src="https://opencollective.com/sit/sponsor/1/avatar.svg"></a>
<a href="https://opencollective.com/sit/sponsor/2/website" target="_blank"><img src="https://opencollective.com/sit/sponsor/2/avatar.svg"></a>
<a href="https://opencollective.com/sit/sponsor/3/website" target="_blank"><img src="https://opencollective.com/sit/sponsor/3/avatar.svg"></a>
<a href="https://opencollective.com/sit/sponsor/4/website" target="_blank"><img src="https://opencollective.com/sit/sponsor/4/avatar.svg"></a>
<a href="https://opencollective.com/sit/sponsor/5/website" target="_blank"><img src="https://opencollective.com/sit/sponsor/5/avatar.svg"></a>
<a href="https://opencollective.com/sit/sponsor/6/website" target="_blank"><img src="https://opencollective.com/sit/sponsor/6/avatar.svg"></a>
<a href="https://opencollective.com/sit/sponsor/7/website" target="_blank"><img src="https://opencollective.com/sit/sponsor/7/avatar.svg"></a>
<a href="https://opencollective.com/sit/sponsor/8/website" target="_blank"><img src="https://opencollective.com/sit/sponsor/8/avatar.svg"></a>
<a href="https://opencollective.com/sit/sponsor/9/website" target="_blank"><img src="https://opencollective.com/sit/sponsor/9/avatar.svg"></a>

