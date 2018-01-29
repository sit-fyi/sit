[![Build Status](https://travis-ci.org/sit-it/sit.svg?branch=master)](https://travis-ci.org/sit-it/sit)

# SIT Issue Tracker

SIT (Standalone Issue Tracker, or Simple Issue Tracker, or simply SIT's an Issue
Tracker; or whatever else you like) is a small tool to manage project issue
artifacts on a filesystem. Its (rather simple) goal is to make such artifacts
as bug reports, problem statements, discussions and such surive the lifetime of
project. Instead of depending on a third-party provider (such as GitHub), it
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

Oh, why another issue tracker, let alone file-based? There's GitHub, JIRA,
Trello, Bugzilla, Redmine and hundreds of other trackers, including file-based
ones as well (in various stages of abandonment).

The core motivation for developing SIT was an understanding that these records
of issues aren't just auxiliary. If they are maintained with at least some
degree of responsibility, they carry a lot of important information. Besides
things like backlog or current status, these records provide valuable insight
into decision making by exposing all notes and conversations around particular
changes, defects and such.

The idea was to build a tool that works nicely with SCMs (centralized or not)
but ultimately does not depend on them and can live without one. A tool that
would allow the source code of the project (or whatever else a repository might
carry) to contain this entire body of knowledge, decreasing the risk of its
loss.

With that thought in mind, none of the SaaS or even self-hosted web tools would
do the job. Some issue trackers depend on Git or an array of SCMs to accomplish
some of their goals. Many have been abandoned (sad truth!).

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

## Overview

### Repository

Repository is a collection of issues. By default, such directory is called
`.sit` and is found by the tooling by scanning the working directory and upwards
until such directory is found.

Each repository contains `config.json` file which contains its configuration.
The convention of this file is to contain all configurable items to avoid
potential breakage of behaviour if some defaults are to be changed going
forward.

One can initialize a SIT repository in their working directory using `sit init`
command. It will create `.sit` in the working directory.

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
are allowed to describe the same record from different perspectivees (could be
a generic issue description submission, such as `.type/DescriptionChanged`,
and can also be seen as a problem statement, for example, `.type/ProblemStated`)

This allows to establish non-exclusive ordering of records and allow records to
be prepared independently without having to synchronize their naming (for
example, in a fork or over email). By convention, if there is more than one of
the last records, when a new record needs to be added, it is appended to all of
them.

A record is represented by a directory named after its deterministic hash (by
default, Base32-encoded), with the content hashed inside of this record.

A record is typically linked to a previous record via previous record's hash,
unless this record is considered to be one of the first records. A record can be
linked ot more than one previous record, effectively "joining" the threads.
These links are represented by empty files
`.prev/[previous-record-id-using-the-same-encoding]`.

This approach allows us to preserve the totality of the changes occured, without
having to rely on SCM capabilities. That's right, even if one is to lose the
actual repository, but to keep the source code tree, nothing will be lost on
SIT's side. The directory layout described is chosen in favour of plain text
append-only files for two reasons:

1. It's far more merge-friendly (one wouldn't incur merge conflicts)
2. It's an easier mechanism for managing record's supplemental files (no need to both include files and list them in
   a file, just including a file is sufficient)
  
Below is a list of record file conventions:
  
| Filename   | Description                                                                                                        | Notes                                                                                                |
|------------|--------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------|
| .type/TYPE | Record type. Open-ended, unknown types must be ignored.                                                            | Required, more than one entry is allowed. Case-insensitive to allow for case-insensitive filesystems |
| .prev/ID   | Link to a previous record ID.                                                                                      | Optional, more than one entry is allowed                                                             |
| .timestamp | ISO-8601 timestamp, always with zero UTF offset (`YYYY-MM-DDTHH:mm:ss.sssZ`).                                      | Optional but generally encouraged                                                                    |
| .authors   | List of record authors (one per line, `John Doe <john@doe>` format is recommended, `John Doe` is also acceptable ) | Recommended                                                                                          |

You can create a record using `sit record <issue id> [FILE]..` command.

### Reducers

Reducer is a very important concept in SIT. By themselves, records are cool but of little
practical value as they don't allow us to observe the current state of any issue but
only its history. The naming comes from [fold, or reduce function](https://en.wikipedia.org/wiki/Fold_(higher-order_function))

In a nutshell, a reducer takes current state, an item to process and returns an update state:

```
Reducer(State, Item) -> State1;
```

In practicular terms, a reducer takes a state of the issue (a JSON object), and a record
and returns an update JSON object with the state of the issue. In order to produce a meaningful
representation of an issue, we must iterate records in order to get a valid result. One of the
interesting features here is the ability to process records up to a certain point to see how
an issue looked back then.

The result of this reduction can be used as-is or be used in a user interface to produce a
useful rendering of it.

Currently, the core dictionary processed by SIT is very small (but it is expected to grow):

| Type           | Description                     | Files                                                | State effect                      |
|----------------|---------------------------------|------------------------------------------------------|-----------------------------------|
| SummaryChanged | Changes issue's summary (title) | * `text` - a UTF-8 string, expeced to be a one-liner | Updates `summary` field           |
| DetailsChanged | Changes issue's details (body)  | * `text` - a UTF-8 string                            | Updates `details` field           |
| Closed         | Closes issue                    |                                                      | Updates `state` field to `closed` |
| Reopened       | Reopens issue                   |                                                      | Updates `state` field to `open`   |

One can look at the state of the issue with the `sit reduce <issue id>` command.

Currently, the only way to add reducers is through adding them to `sit-core` (or building a third-party library). However,
adding [sandboxed] scrpiting languages backends (so that reducers can be added per-repository or per-user easily) is also
planned.


## License

SIT is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See LICENSE-APACHE and LICENSE-MIT for details.
