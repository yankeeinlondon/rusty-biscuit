# Core Structs and Features

## Overview

We want to have a fairly consistent set of utilities for handling PDF, TOML, and YAML files. For each we want to have a features:

- Read file and parse into some internal representation
- Export content in other formats
- Provide as many trait implementations of From<T> or TryFrom<T> or anything else which will aid callers in using these utility structs.

## PDFs

We will define a `Pdf` struct which will form the foundation of our PDF based utilities. It will provide:

- `as_markdown()` to convert the content into a markdown format (all text but also ideally inline images too)
    - try to preserve metadata in the PDF like links, formatting, etc.
- `as_text()` to convert content into plain text
- `toc()` provides the table-of-contents of the document
- `new()` will take a file path but we'll also have TryFrom from any variant of a string (to be converted to a file path)

## TOML

We will define a `Toml` struct which will form the foundation of our TOML based utilities: It will provide:

- `as_json()`
- `as_yaml()`
- `validate()`


## YAML

We will define a `Yaml` struct which will form the foundation of our YAML based utilities: It will provide:

- `as_json()`
- `as_toml()`
- `validate()`
