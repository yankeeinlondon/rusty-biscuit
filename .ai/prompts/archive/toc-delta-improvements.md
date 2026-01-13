# the `--delta` option in the `md` CLI

This document attempts to describe all of the detections we'd expect when using the `--delta` flag in the **md** CLI. Currently we have some problems with the current implementation.

- The biggest issue is it's ignoring "punctuation" changes. This is not expected or desired.

The good news is we've recently added two useful new modules to the shared library in this monorepo:

1. **isolate** [`./shared/src/isolate/*`]
2. **interpolate** [`./shared/src/interpolate/*`]

These capabilities will be particular useful when we

