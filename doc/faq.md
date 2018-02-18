# Frequently Asked Questions

## Why?

Because we can?

Seriously, though, there is a number of reasons. One of the original
motivations was to couple issues (as a form of communication
and record) with the code it is about so that for any revision
of that we can see the snapshot of the entire space of issues
at precisly that time.

Another reason was to remove dependencies on external services or products. 
Those became extremely popular recently. They are convenient, but
you lose direct control over your data. Often time it is hard
to take it offline and work on it in a meaningful way. Even when
using self-hosted products, they are still hosted somewhere. SIT
in its entirety (both the tool and the issues under its management)
can be stored on a USB flash drive.

Lastly, but not least importantly, because things can be simple.

## What about "normies" (less technically-savvy)? How can they use it?

It depends on your model of sharing issues. If you share them over, say,
Dropbox or something similar, it'll work just fine -- they just start
one program (`sit-web`) and use it through a browser. No extra steps
required.

If your workflow involves an SCM (such as Git), the setup will be
more involved (at least for now). They'd have to be tought to use
some GUI client for the SCM of your choice.

## Is there a way to host a shared public/private instance of sit-web?

Not yet. This is an interesting topic but we are not there yet.

## Why there are so many "hidden" files starting with dot?

Great question. (Also, kudos for looking inside!)

The initial motivation for the distinction between "normal" and
".other" files in `.sit` directory was to distinguish between
files that are "meta" (those would be dot-files) and the actual data.

To give you an example, in records, you will often see files like
`.type/Commented` or `.timestampt` -- these files are about the record,
while `text` or `git/0001-Problem-everything-is-ok.patch` are **the content**.

Same idea applies to directories directly under `.sit`: `issues` contains issues
and `.issues` contains information that's meta to it, like `.issues/filters`.

Hope this makes sense.

## What about removing sensitive information?

You might have read that SIT is immutable, therefore, you might
be stuck with sensitive information sitting in an issue. Well,
it is immutable to the same degree Git is. In a normal course
of operations, it is. However, if it comes to that, there's
`sit rebuild` command that allows to alter everything. Keep in mind
that if any of your team members PGP-signed their records, those
signatures will be invalidated as a part of the process. Not out
of malicious intent, though -- simply because the content
will change (issues link to each other and their IDs are in fact
hashes of their content).

## How about permissions?

Currently, SIT introduces no conventions for permissions. It is
definitely expected that some reduction primitives will be developed
for this over time. However, the important point here is that
the final state of the issue is a result of reduction of all
its records over the state, so it is always possible to filter out
unwanted changed. All changes are saved, and therefore, actions
are not irreversible. 

## Is there a way to import issues from GitHub?

Not yet, but this is definitely in the pipeline.

## What browsers are supported?

Both `sit-web` and [SIT's website](http://sit-it.org) rely on Web Components
heavily. The best browser (and the most tested in our case) with respect
to that is Chrome/Chromium. Next one is, apparently, Opera. Then, it is
Safari, followed by Firefox and Edge. We have received reports of malfunctioning
components in Firefox but were unable to reproduce them in Firefox 58 just yet
(please note, however, that performance in Firefox is severely impaired comparing
to, say, Chrome). 

Our hope and expectation is that Web Components' take-up will be steady and
the support will be more even.

Could we have used something "time-proven" instead? Yes, of course. However,
in this case it was decided that it was worth a short. Web Components give
some interesting customization capabilities and work (except when they don't
in browsers in weaker Web Components support) in the browsers as is
without any backend processing, which is useful in our case. Time will tell.

