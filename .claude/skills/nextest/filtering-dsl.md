# Filtering DSL (filtersets)

Nextest supports a filter expression language to precisely select tests.

## Core predicates

* `all()` — everything
* `test(<pattern>)` — match test names (often regex-like, e.g. `/^foo::/`)
* `package(<name>)` — tests from a Cargo package
* `kind(<type>)` — lib/bin/example/etc. (depending on suite)
* `platform(<spec>)` — target platform filtering

## Boolean logic

* `and`, `or`, `not`

## Examples

* All tests in one package:
  * `cargo nextest run 'package(my-crate)'`
* Select by name:
  * `cargo nextest run 'test(my_test_name)'`
* Regex-y selection:
  * `cargo nextest run 'test(/integration.*/)'`
* Exclude slow module:
  * `cargo nextest run 'not test(/^slow::/)'`
* Combine:
  * `cargo nextest run 'package(integration) and not test(/^flaky::/)'`

## Platform filtering

Common case:

* `cargo nextest run 'package(my-crate) and platform(x86_64-unknown-linux-gnu)'`

Cross compilation nuance: understand host vs target. In config overrides, you may need:

````toml
[[profile.default.overrides]]
platform = { host = "cfg(unix)", target = "aarch64-apple-darwin" }
````

If filters don’t behave as expected, confirm which platform (host/target) you meant.

---