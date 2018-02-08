[![Gitter chat](https://badges.gitter.im/sit-it/Lobby.png)](https://gitter.im/sit-it/Lobby)
[![Build Status](https://travis-ci.org/sit-it/sit.svg?branch=master)](https://travis-ci.org/sit-it/sit)
[![Windows Build status](https://ci.appveyor.com/api/projects/status/0iv6ltgk3pa122hx?svg=true)](https://ci.appveyor.com/project/yrashk/sit)


# SIT Issue Tracker

SIT (Standalone Issue Tracker, or Simple Issue Tracker, or simply SIT's an Issue
Tracker; or whatever else you like) is a small tool to manage project issue
artifacts on a filesystem. Its goal is to make such artifacts
as bug reports, problem statements, discussions (and others) surive the lifetime of
the project. Instead of depending on a third-party provider (such as GitHub), it
ensures that its database is not going to be lost if a third-party provider goes
out of business, loses data, or if you fall in love with a new SCM and decide to
migrate over.

Its core properties are:

* All artifacts are stored in a file system
* All records are immutable and ordered
* It's SCM-agnostic (and, in fact, will work without one just fine)
* Its layout is purposefully merge-friendly (who likes merge conflicts?)
* It's extensible

SIT's design allows us to have some interesting features, such as:

* Branch-local issues (useful for managing branch-local todo lists, for example)
* Branch-specific issue states (for example, "fixed in the branch, but not in the master")

## Project Status

Is it ready to use? Only if you are brave enough or just want to play with it.
The core prototyping is done, but it still lacks a well-defined dictionary
of record types and a decent UI. Hopefully not for long, though!

## Motivation

Oh, why another issue tracker, let alone file-based, you ask? There's GitHub, JIRA,
Trello, Bugzilla, Redmine and hundreds of other trackers, including file-based
ones as well (in various stages of abandonment).

The core motivation for developing SIT was an understanding that these records
of issues aren't auxiliary. If they are maintained with at least some
degree of responsibility, they carry a lot of important information. Besides
things like backlog or current status, these records provide valuable insight
into decision making by exposing all notes and conversations around particular
changes, defects or issues.

The idea was to build a tool that works nicely with SCMs (centralized or not)
but ultimately does not depend on them and can live without one. A tool that
will allow the contents of the project to contain this entire body of knowledge,
decreasing the risk of its loss.

Hence the experiment to build SIT.

## Build Instructions

As SIT is not currently distributed in any package managers, you'd have to
build it manually. Make sure you have Rust 1.23 or higher installed and run
this:

```shell
cargo build --release
```

The resulting binary can be found at `./target/release/sit`. Put it in your
`PATH` to be able to access it in your environment.

## Got questions, bug reports, etc?

SIT's is using SIT for tracking issues (duh!) and because of this, GitHub
issues are turned off. It's a good excuse to try out SIT if you have an
issue to file!

The user experience is very barebones at the moment (until we'll get something
better) but it does the job (more or less).

### Preparation

Before using SIT for the first time, please create `$HOME/.config/sit/config.json`
file and record your information:

```json
{
  "author": {
    "name": "Your Name",
    "email": "your@email"
  }
}
```

Currently, SIT will refuse to start without this information specified as it is used
to record authorship in issues.

It is also **highly recommended** to sign your records. To enable this, add the following
to your config file:

```json
"signing": {
   "enabled": true,
   "key": "key you are going to use"
}
```

### Listing issues

By default, `sit issues` will list all issues by their IDs. However,
this is hardly practical if you just want to see a list of issues you want to be able
to process quickly, or if you want to search for specific kinds of issues.

Luckily, sit integrates [JMESPath](http://jmespath.org) filter and querying. This allows
us to achieve a lot.

For example, we can list all issues with their ID and summary using processing query (`--query/-q`):

```
$ sit issues -q "join(' | ', [id, summary])"
a59dfc1e-cf88-4c18-a728-23baab41f7d2 | Problem: no way to discuss issues
efc6b084-db52-4d20-80b9-20112f679660 | Problem: sit requires to specify authorship
885a8af0-22ff-455c-89a6-68a13597dd53 | Problem: SIT is not very ergonomic for day-to-day use
6913711b-34ab-471f-9e83-77a719e0697a | Problem: no record authorship preserved
09274126-7d3c-4a32-9338-a5501e1bfb84 | Problem: issue state does not account for unauthorized editing
```

(The above output is just an example so that you can see what it can produce)

If you want to filter out closed issues, a filtering query (`--filter/-f`) will come in handy:

```
$ sit issues -f "state != 'closed'" -q "join(' | ', [id, summary])"
```

You can list issues in their entirety as well:

```
$ sit issues -q @
```

But of course, this is not ideal as you'd have to remember and re-type
specific queries or filters to address your needs. For this, named filters
and queries should be used.

They can be defined either per SIT-repository, or in sit config. In repository,
filters they are defined with files named `.issues/filter/NAME`, and
queries are defined with files named `.issues/queries/NAME. Their content
should be the expression to be evaluated.

If you want to define filters or queries in your sit config instead (so it is local
to you, but not shared with other SIT repository users), you can
specify them in `issues.filters` and `issues.queries` properties:

```json

{
 "isues": {
    "queries": {
       "overview": "join(' | ', [id, summary])"
    },
    "filters": {
       "not-closed": "state != 'closed'"
    }
 }
} 
```

These queries can be used with the `--named-query/-Q` flag and filters
with `--named-filter/-F` flag.

### Open an issue

1. Run `sit issue`, note the ID generated by it
2. Edit temporary `text` file to prepare a one-line summary (title).
   It is important to name the file `text` and not something else.
   Within SIT project we kindly request to use the "problem statement"
   summary as in: `Problem: something doesn't work` whenever possible.
3. Take ID from the first step and run `sit record -t SummaryChanged <id> text`
4. Edit temporary `text` file to prepare details.
   Provide detailed information for your issue so that others can fully
   understand it. It is a good etiquette to have one or a few paragraphs.
5. Take ID from the first step and run `sit record -t DetailsChanged <id> text`
6. You can check if everything is correct by running `sit reduce <id>`.
   It will show the current state of the issue as a JSON.
   
### Comment on an issue

1. Edit a temporary `text` file to prepare your comment.
   It is important to name the file `text` and not something else.
2. Take ID of your issue and run `sit record -t Commented <id> text`

### Send it to upstream

Now that your issue is recorded locally, you can send it to this repository:

1. Create a branch (as a convention, you can use your issue ID as a branch name)
2. Add new files in `.sit` and commit them. Commit message can be simply "Added issue ISSUE-ID"
   or, say, "Commented on issue ISSUE-ID"
3. Push it out to the inbox: `GIT_SSH_COMMAND="ssh -i sit-inbox" git push git@git.sit-it.org:sit-it/sit-inbox.git <branch>`
4. If the commit only contains new records (nothing else permitted!) the inbox
   will accept the push and immediately push it out to sit's master repository on GitHub.
   Otherwise, the push will be rejected.
   
To further simplify the process of sending records to the upstream,
it's highly recommended to add a remote (such as `issues`) for `git@git.sit-it.org:sit-it/sit-inbox.git`
and add this to your `~/.ssh/config`:

```
host git.sit-it.org
  HostName git.sit-it.org
  IdentityFile /path/to/sit/repo/sit-inbox
  User git
```

This way, pushing out, will be as nice as `git push issues <branch>`

### Getting updates

You will get all issue updates when you fetch this git repository.

### Preparing a merge request

Please refer to [CONTRIBUTING](https://github.com/sit-it/sit/blob/master/CONTRIBUTING.md#preparing-a-merge-request) for the instruction.

## Overview

<center>
<p align="center">
<img src="doc/overview.png" width="266" height="543">
</p>
</center>

### Repository

Repository is a collection of issues. By default, such directory is called
`.sit` and is found by the tooling by scanning the working directory and upwards
until such directory is found.

Each repository has `config.json` file which contains its configuration.
The convention for this file is to contain all configurable items to avoid
potential breakage of behaviour if some defaults are to be changed going
forward.

One can initialize a SIT repository in their working directory using `sit init`
command. It will create `.sit` directory.

### Issue

Issue is a topic or a problem for debate, discussion and resolution (aka "ticket")
and is represented by a uniquely named directory within a repository. While some
issues might be named manually (might be a great way to establish some
conventions), it is generally recommended that a globally unique identifier is
generated for every new issue (such as UUID, which is the default employed by
SIT)

Because of SIT's extensible nature, issue can be also be used to represent a
wild variety of entities. For example, a Kanban board with its records
representing movement of other issues into, across and out of the board.

Each issue is comprised of zero or more records (although issues with zero
records aren't very practical).

You can create a new issue using `sit issue` command and you can list IDs
of all issues using `sit issues`.

### Record

Record is an immutable collection of files. Record is identified by a
deterministic hash of its content (for each file, hash relative file name and
then hash its content to get a cumulative hash). A record is typically linked to
a previous record via previous record's hash, unless this record is considered
to be one of the first records.

Record is used to represent an "event" that is applied to its container. For
example, a record might represent changing an issue's title, stating a problem
or adding an attachment (or just about anything else). By convention, `.type/TYPE`
file within a record is used to describe the type of the record. Multiple types
are allowed to describe the same record from different perspectives (could be
a generic issue description submission, such as `.type/DescriptionChanged`,
and can also be seen as a problem statement, for example, `.type/ProblemStated`)

A record is represented by a directory named after its deterministic hash (by
default, Base32-encoded), with the content hashed inside of this record.

A record is typically linked to a previous record via previous record's hash,
unless this record is considered to be one of the first records. A record can be
linked to more than one previous record, effectively "joining" them.
These links are represented by empty `.prev/[previous-record-id]` files.

This allows to establish non-exclusive ordering of records and allow records to
be prepared independently without having to synchronize their naming (for
example, in a fork or over email). By convention, if there is more than one of
the last records, when a new record needs to be added, it is appended to all of
them.

Below you can see an artificial example that shows ordering of records:

<center>
<p align="center">
<img src="doc/records.png" width="525" height="543">
</p>
</center>

(Here `H5JFAN2QSAPYX34SGTK66YFUTFS55V2` is the first record and `56AGOFFETK2KFQP2FX5OF5B2RULCAUB2`
is the last one and it "joins" `ORV3F2MEBQEDHIM4A6ATLQJKQ7OMEMT6` and `ORV3F2MEBQEDHIM4A6ATLQJKQ7OMEMT6`)

This approach allows us to preserve the totality of the changes occured, without
having to rely on SCM capabilities. That's right, even if one is to lose the
actual repository, but to keep the source code tree, nothing will be lost on
SIT's side. The directory layout described is chosen in favour of plain text
append-only files for two reasons:

1. It's far more merge-friendly (one wouldn't incur merge conflicts)
2. It's an easier mechanism for managing record's supplemental files (no need to both include files and list them in
   a file, just including a file is sufficient)
  
Below is the list of some record files conventions:
  
| Filename   | Description                                                                                                        | Notes                                                                                                |
|------------|--------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------|
| .type/TYPE | Record type. Open-ended, unknown types must be ignored.                                                            | Required, more than one entry is allowed. Case-insensitive to allow for case-insensitive filesystems |
| .prev/ID   | Link to a previous record ID.                                                                                      | Optional, more than one entry is allowed                                                             |
| .timestamp | ISO-8601 timestamp, always with zero UTF offset (`YYYY-MM-DDTHH:mm:ss.sssZ`).                                      | Optional but generally encouraged                                                                    |
| .authors   | List of record authors (one per line, `John Doe <john@doe>` format is recommended, `John Doe` is also acceptable ) | Recommended                                                                                          |
| .signature | ASCII PGP signature of the encoded hash of the record without this file (`gpg --sign --armor`)                     | Recommended                                                                                          |

You can create a record using `sit record <issue id> [FILE]..` command.

### Reducers

Reducer is a very important concept in SIT. By themselves, records are cool but of little
practical value as they don't allow us to observe the current state of any issue but
only its history. 

The naming comes from [fold, or reduce function](https://en.wikipedia.org/wiki/Fold_(higher-order_function))

In a nutshell, a reducer takes current state and an item to process and returns an update state:

```
Reducer(State, Item) -> State1;
```

In practicular terms, a reducer takes a state of the issue (a JSON object), and a record
and returns an updated JSON object with the state of the issue. In order to produce a meaningful
representation of an issue, we must iterate records in order to get a valid result. One of the
interesting features here is the ability to process records up to a certain point to see how
an issue looked back then.

The result of this reduction can be used as-is or in a user interface to produce a
comprehensible rendering of it.

Currently, the core dictionary processed by SIT is very small (but it is expected to grow) and
can be found in [documentation](doc/dict).

One can look at the state of the issue with the `sit reduce <issue id>` command.

Currently, the only way to add reducers is by adding them to `sit-core` (or building a third-party library). However,
adding [sandboxed] scripting languages backends (so that reducers can be added per-repository or per-user easily) is also
planned.


## License

SIT is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See LICENSE-APACHE and LICENSE-MIT for details.

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
