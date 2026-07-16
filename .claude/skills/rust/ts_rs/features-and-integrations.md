# Features & Integrations

Enable feature flags to support popular external crate types (implementations are feature-gated due to Rust orphan rules).

Common:

* `chrono`: maps chrono date/time types (often to TS `string`)
* `uuid`: maps `Uuid` to TS `string`
* `serde-json`: supports `serde_json::Value`
* `url`: maps `url::Url` to TS `string`
* `bigdecimal`, `bson`, `bytes`, `indexmap`, `ordered-float`, `semver`, `smol-str`, `heapless`
* `format`: formats generated TS output

Troubleshooting hint:

* If a field’s type fails to derive TS, check whether a feature flag exists for that crate/type.

---