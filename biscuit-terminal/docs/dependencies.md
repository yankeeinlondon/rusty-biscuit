# biscuit-terminal Dependencies

## Structure

- `biscuit-terminal/lib/Cargo.toml` - Terminal detection and rendering library
- `biscuit-terminal/cli/Cargo.toml` - Terminal inspector CLI (`bt`)

## Production Dependencies

### Base64 Encoding

- [base64](https://github.com/marshallpierce/rust-base64) _v0.22_

    _Fast base64 encoding/decoding for image protocol data._

    _Tags: encoding, base64_

### Date & Time

- [chrono](https://github.com/chronotope/chrono) _v0.4_

    _Date and time library providing timezone-aware types and operations._

    _Tags: date, time, timezone_

### Error Handling

- [thiserror](https://github.com/dtolnay/thiserror) _v2.0_

    _Derive macro for std::error::Error trait._

    _Tags: errors, macros_

### Filesystem

- [ignore](https://github.com/BurntSushi/ripgrep) _v0.4_

    _Fast recursive directory iterator respecting .gitignore and file type filters. Used by the FileSystem component for gitignore-aware directory tree rendering._

    _Tags: filesystem, gitignore, filtering_

- [tempfile](https://github.com/Stebalien/tempfile) _v3_

    _Secure cross-platform temporary file/directory creation for mermaid rendering._

    _Tags: filesystem, temporary_

### Config Parsing

- [plist](https://github.com/ebarnard/rust-plist) _v1_

    _Apple property-list (XML and binary) reader. Used by the app-metadata value
    extractor to read iTerm2 / Apple Terminal config settings. This is the only
    structured-format parser added directly to biscuit-terminal (spec §6/§10);
    TOML/YAML/JSON5 go through `biscuit-file` instead._

    _Tags: config, plist, macos, parsing_

### Image Processing

- [image](https://github.com/image-rs/image) _v0.25_

    _Image decoding/encoding for terminal image rendering._

    _Tags: image, graphics_

- [viuer](https://github.com/atanunq/viuer) _v0.11_

    _Terminal image viewer supporting Kitty and iTerm2 graphics protocols._

    _Tags: terminal, images, graphics_

### Logging & Tracing

- [tracing](https://github.com/tokio-rs/tracing) _v0.1_

    _Structured, async-aware logging framework with spans and events._

    _Tags: logging, tracing, observability_

- [tracing-subscriber](https://github.com/tokio-rs/tracing) _v0.3_

    _Utilities for implementing and composing tracing subscribers._

    _Tags: logging, tracing, formatting_

### Regular Expressions

- [regex](https://github.com/rust-lang/regex) _v1.11_

    _Regular expression library for escape code analysis and text processing._

    _Tags: regex, parsing_

### URL Handling

- [url](https://github.com/servo/rust-url) _v2.5_

    _Standards-compliant file URL construction for filesystem hyperlinks,
    including Windows drive-letter and separator normalization._

    _Tags: url, filesystem, hyperlinks_

### Serialization

- [serde](https://github.com/serde-rs/serde) _v1.0_ [docs](https://serde.rs)

    _Industry-standard serialization framework._

    _Tags: serialization_

- [serde_json](https://github.com/serde-rs/json) _v1.0_

    _Fast JSON serialization/deserialization._

    _Tags: json, serialization_


### Terminal

- [terminal_size](https://github.com/eminence/terminal-size) _v0.4_

    _Cross-platform terminal size detection._

    _Tags: terminal, dimensions_

- [termini](https://crates.io/crates/termini) _v1.0_

    _Terminal capability database access._

    _Tags: terminal, capabilities_

### Text Processing

- [unicode-width](https://github.com/unicode-rs/unicode-width) _v0.2_

    _Unicode display width calculation for proper text alignment._

    _Tags: unicode, text, width_

### Localization

- [unic-langid](https://github.com/nickel-org/unic-locale) _v0.9_

    _Unicode Language Identifier parsing for locale detection._

    _Tags: unicode, locale, i18n_

## Workspace Dependencies

- [biscuit-file](../../biscuit-file/lib) _v0.1.0_ (features: `toml`, `yaml`, `json5`; `default-features = false`)

    _In-repo format layer used by the app-metadata value extractor. Parses
    TOML/YAML/JSON5 config files and normalizes each to a single
    `serde_json::Value`, so one shared dot-path resolver reads all structured
    formats. Chosen over depending on `toml` / `serde_yaml_ng` / `json-five`
    directly (spec §6). `default-features = false` keeps its heavy PDF/gix tree
    out. No dependency cycle — `biscuit-file` does not depend on biscuit-terminal._

    _Tags: workspace, config, parsing_

- [biscuit-visualized](../../biscuit-visualized) _v0.1.0_

    _Shared visualization library for Mermaid diagrams and graph rendering. Delegates to mermaid-rs-renderer and layout-rs._

    _Tags: workspace, visualization_

- [sniff](../../sniff/lib) _v0.1.0_

    _Git/repo/monorepo detection used by `Terminal::new()`. The CLI also uses
    `sniff::programs::find_program_with_source` for install detection in
    `bt about [APP]`; the library itself stays sniff-free._

    _Tags: workspace, detection_

## Platform-Specific Dependencies

### Unix

- [libc](https://github.com/rust-lang/libc) _v0.2_

    _Raw FFI bindings to platform libraries for terminal ioctl operations._

    _Tags: ffi, unix, system_

## Development Dependencies

### Testing

- [assert_cmd](https://github.com/assert-rs/assert_cmd) _v2_

    _CLI testing framework for integration tests._

    _Tags: testing, cli_

- [predicates](https://github.com/assert-rs/predicates-rs) _v3_

    _Assertion predicates for test output matching._

    _Tags: testing, assertions_

- [proptest](https://github.com/proptest-rs/proptest) _v1.5_

    _Property-based testing framework._

    _Tags: testing, property-based_

- [serial_test](https://github.com/palfrey/serial_test) _v3_

    _Test isolation for environment variable manipulation._

    _Tags: testing, isolation_

- [tempfile](https://github.com/Stebalien/tempfile) _v3_

    _Temporary files for test fixtures._

    _Tags: testing, filesystem_

## Optional Dependencies

### Test tiers

- `terminal-tests` enables the real-terminal harness and tier gate for the
  library and CLI integration targets.
- `browser-tests` enables the headless-browser harness used by computed-style
  tests. Browser-only items are cfg-gated inside the mixed parity target.

Both are disabled for ordinary local L1 and enabled by their explicit tier
recipes and CI feature policy.

### CLI Support

- [clap](https://github.com/clap-rs/clap) _v4_ (feature: `clap`)

    _Command-line argument parsing with derive support for ValueEnum._

    _Tags: cli, arguments, optional_
