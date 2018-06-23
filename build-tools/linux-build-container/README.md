This container provides a convenient way to build and test sit in isolation
from the local environment (Linux-specific). This is particularly useful for
running integration tests that might inadvertently read user configuration and
fail because of that (or introduce undesired behaviour such as requiring gnupg
signatures and triggering gpg-agent popups) ir building releases.

A suggested way to run such isolated environment would be:

```
docker run -v /host/path/to/sit/repo:/sit -e CARGO_TARGET_DIR=/sit/test_target -ti <image tag>
```

