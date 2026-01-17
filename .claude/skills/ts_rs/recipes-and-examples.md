# Recipes & Examples

## Full-stack API DTO with Serde rename parity

````rust
use serde::{Serialize, Deserialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ApiUser {
    pub user_id: i32,
    pub display_name: Option<String>,
}
````

Result: TS fields match JSON casing.

## Optional fields strategy

If your API omits `None` fields (common in JSON):

* Use `#[ts(optional_fields)]` at container level, or `#[ts(optional)]` per field.

## Flatten nested structs

````rust
use ts_rs::TS;

#[derive(TS)]
pub struct Address { pub street: String, pub city: String }

#[derive(TS)]
#[ts(export)]
pub struct Person {
  pub name: String,
  #[ts(flatten)]
  pub address: Address,
}
````

## Discriminated union response type

````rust
use ts_rs::TS;

#[derive(TS)]
#[ts(export, tag = "type")]
pub enum ApiResponse<T> {
  Success { data: T },
  Error { message: String },
}
````

## One “export bindings” test for a crate

Create `tests/export_ts.rs` or a `#[cfg(test)]` module and call:

* `RootType::export_all_to("./frontend/src/types")`

This exports the root type and all dependencies, keeping the workflow deterministic.