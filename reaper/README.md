# Reaper

**Reaper** is a Rust Library (./lib) and CLI (./cli) dedicated to extracting context out of web pages.

## CLI

### URL Analysis

```sh
# Provides summary information about the URL
reaper https://somewhere.com
# Provides a more complete summary about the URL
reaper https://somewhere.com --deep
```

**Note:** Analysis results are always 

### Site Analysis



## Library

Like other package areas in **rusty-biscuit**, Reaper's library provides all of the important business logic which supports the CLI but also allows for other libraries to leverage all of the core functionality themselves.
