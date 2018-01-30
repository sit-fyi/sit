Our goal is to encourage frictionless contributions to the project. In order to
achieve that, we use Unprotocols' [C4 process](https://rfc.unprotocols.org/spec:1/C4)
as an inspiration. Please read it, it will answer a lot of questions. Our goal is to
merge pull requests as quickly as possible and make new stable releases regularly. 

In a nutshell, this means:

* We merge pull requests rapidly (try!)
* We are open to diverse ideas
* We prefer code now over consensus later

It is highly recommended to watch [Pieter Hintjens' talk on building open
source communities](https://www.youtube.com/watch?v=uzxcILudFWM) as well as
read his [book on the same
matter](https://www.gitbook.com/book/hintjens/social-architecture/details).

# Submitting an issue

According to C4's [development process](https://rfc.unprotocols.org/spec:1/C4#24-development-process),
the issue should describe a documented and provable. What this means is that an
issue should trive to have a clear, understandable problem statement. Just like
a patch, it SHOULD be titled "Problem: ..." and have a detailed description
describing evidence behind it, be it a bug or a feature request, or a longer
term "exploratory" issue.

Unlike C4, we're not using GitHub (or any other well-known platform) for issues.
Instead, we're using SIT itself. Please refer to [README](https://github.com/sit-it/sit#open-an-issue)
section that covers this process.

# Preparing a patch

According to [patch requirements](https://rfc.unprotocols.org/spec:1/C4#23-patch-requirements),
the patch should be a minimal and accurate answer to exactly one identified and
agreed problem. A patch commit message must consist of a single short (less
than 50 characters) line stating the problem ("Problem: ...") being solved,
followed by a blank line and then the proposed solution ("Solution: ...").

```
Problem: short problem statement

Optional longer explanation of the problem that this patch
addresses, giving necessary details for the reader to be
able to understand it better.

Solution: explanation of the solution to the problem. Could
be longer than one line.
```

Typically, patches are expected to be submitted over a pull request, but
if your circumstances preclude you from doing so, you can reach out to
any maintainer privately by e-mail or any other available means and
send the patch that way.
