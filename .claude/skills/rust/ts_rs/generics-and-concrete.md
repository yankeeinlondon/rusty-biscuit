# Generics, Concretization, and Bounds

## Exporting generic types

ts-rs can export generics directly:

````rust
#[derive(ts_rs::TS)]
#[ts(export)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: u64,
}
````

## Concretizing generics for a stable API surface

If your frontend only uses specific instantiations, export a concrete version:

* Use `#[ts(concrete(T = String))]` on a generic type
* Or define a `type` alias/new wrapper struct in Rust dedicated to export

Example pattern:

````rust
#[derive(ts_rs::TS)]
#[ts(export)]
#[ts(concrete(T = String))]
pub struct Envelope<T> {
    pub data: T,
}
````

## When you need explicit bounds

Complex generics may require:

* `#[ts(bound = "T: ts_rs::TS")]`

## Associated types limitation

TypeScript doesn’t have Rust associated types in the same way; if a generic parameter implies associated types, you may need:

* Separate DTO types
* `#[ts(concrete(...))]` to fix the associated type to a concrete exported type

---