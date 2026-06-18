---
name: reaper
description: details on the Reaper library and CLI
---


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

- like other package areas in **rusty-biscuit**, Reaper's library provides all of the important business logic which supports the CLI but also allows for other libraries to leverage all of the core functionality themselves.
- **Reaper** tries to do as much as it can _deterministically_ but when a _non-deterministic_ process (e.g., leveraging an LLM or Agent call) it uses an adapter pattern where the calling code must provide an adapter for this functionality to be available.

    ```rust
    let page = WebPage::from("https://somewhere.com").use_adaptor(adaptor);
    ```

    The `InferenceAdapter` trait is provided in the **biscuit-contract** library which is a lightweight
    library that allows any package which wants to provide inference capabilities to pass in an adapter
    which implements this trait.
