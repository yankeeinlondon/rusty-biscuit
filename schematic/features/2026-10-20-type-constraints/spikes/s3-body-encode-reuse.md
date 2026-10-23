# Spike S3 - Body encode reuse (read-only)

## Design paragraph for Phase 5
No runtime encoder change is needed: `RequestBody::Json` carries an already-serialized `String`
and the runtime copies it verbatim, so the reuse lives entirely in generated `into_parts`. For a JSON
body whose plan has rules, emit `let body_value = serde_json::to_value(&self.body).map_err(|_| InvalidArgument::new("the request body cannot be serialized as JSON"))?;`,
then `PLAN.check(&body_value, &mut violations)` (params first, then body), convert a non-empty
`Violations` to `InvalidArgument` and return `Err` before building `RequestParts`, else
`RequestBody::Json(serde_json::to_string(&body_value).map_err(..same..)?)`. The checked `Value` is
exactly the tree whose text is sent; there is no second walk of the typed model. When a body has no
rules, emit the current `serde_json::to_string(&self.body)` token stream unchanged (byte-identical).
Because every send path funnels through `into_parts`, this one place covers `.build()`, await,
`.full()`, `client.send(request)`, `send_raw`, and pager first pages with zero transport sends. Forms
and text bodies are checked on their typed fields in `into_parts` before `into_form_parts`/`into_form_pairs`/`into_bytes`;
a constrained multipart JSON field needs a fallible generated conversion (see Q4).

## Q1 - How JSON bodies become bytes today
- Generator: `gen/src/codegen/request_structs/into_parts.rs:47-53` emits
  `RequestBody::Json(serde_json::to_string(&self.body).map_err(|_| InvalidArgument::new("the request body cannot be serialized as JSON"))?)`.
  Committed output matches: `schema/src/unfolded_circle_core_rest.rs:163-170`.
- Runtime: `RequestBody::Json(String)` (`runtime/src/endpoint.rs`, "A serialized JSON body"); `encode.rs:23-26`
  copies the string into `Bytes` and fixes content type `"application/json"`. No codec abstraction; compact `to_string`.
- `client.rs:1235` `encode::encode(&body)` inside assembly; `client.rs:1236-1240` sets `Content-Type`.

## Q2 - Can a prebuilt Value go through the same path?
Yes, with no runtime change: `serde_json::to_string(&value)` yields the `String` that `RequestBody::Json` takes.
Differences from the direct typed path (only for constrained bodies; all JSON-equivalent or edge cases):
- Key order: `serde_json/src/map.rs:33-36` - `BTreeMap` unless `preserve_order`. In this workspace
  `preserve_order` is enabled ONLY via `tree-sitter`'s **build-dependency** (`cargo tree --workspace -e features -i serde_json`);
  with `resolver = "2"` (root `Cargo.toml:2`) build-dep features do not unify with normal deps, and
  `-e features,normal` shows no `preserve_order`. So Value keys are sorted on the wire. Fragile: any future
  normal dep enabling it flips to insertion order. The evaluator should not rely on `Map` iteration
  order for "deterministic object-key ordering" of violations unless that is documented as sorted-by-BTreeMap.
- `f32` widening: `value/ser.rs:151-153` -> `number.rs:323-329` stores `f as f64`, so `0.1f32` is sent as
  `0.10000000149011612` instead of `0.1`. Real exposure: f32 fields exist in e.g. `definitions/src/unfolded_circle/mod.rs`.
  Recommend the generator note this in diagnostics/docs or flag f32 bodies; numerically same value as the f32.
- `u128`/`i128` above u64/i64 range: `value/ser.rs:134-146` errors without `arbitrary_precision` (direct `to_string` succeeds) -> becomes the existing "cannot be serialized" InvalidArgument.
- Duplicate keys (overlapping `#[serde(flatten)]`): Value collapses to last-wins; direct path writes both.
- `raw_value` is enabled in `schematic/definitions/Cargo.toml:13`: a `Box<RawValue>` field is re-parsed into a Value and re-emitted compact (whitespace/number text normalized).
- `arbitrary_precision` is not enabled anywhere (grep of all Cargo.toml).

## Q3 - Where into_parts errors and how they surface
- Contract: `endpoint.rs:61-70` `fn into_parts(self) -> Result<RequestParts, InvalidArgument>`.
- Generated error sites today: JSON serialize (`into_parts.rs:49-51`), typed path non-finite numbers and cookie merge (`into_parts.rs:19-26` doc).
- `.build()`: `call.rs:134-140` -> `client.check_arguments` (`client.rs:859-872`) -> `into_parts` directly when client invalid (`:864`) else `prepare` (`:866`), unwraps `TransportErrorKind::InvalidArgument` -> plain `InvalidArgument`. `FullCall::build` delegates (`call.rs:171-176`).
- Await / `.full()` / `send_raw`: `call.rs:183-194` and `:201-207` -> `send_unboxed` (`client.rs:757`) -> `execute` (`:948`) -> `prepare` (`:960`); `send_raw` (`:816`, `:824-830`) -> same `execute`.
  `prepare` `client.rs:1077-1080`: `request.into_parts().map_err(|argument| pre_send(TransportErrorKind::InvalidArgument(argument), self.base_meta::<R>()))?` - before assembly, origin check, credentials, and any I/O.
  `.full()`/`send_raw` return that `TransportError`; ordinary await maps it via `?` into `CallError::Transport`.
- `client.send(request)` (`api_client.rs:177`, `client.rs:748`) just builds a `Call`, so it is one of the above.
- Pager: `pager/request.rs:50-52,117-119` and `pager/session.rs:128` wrap the generated `into_parts` result, so pager sends inherit the check too.
- Natural single place: the generated `into_parts`, before `Ok(RequestParts{..})`. Covers all five paths; zero sends guaranteed because `prepare` fails before `execute` does network work. No runtime call-site change needed.

## Q4 - Typed values available for forms and text before encoding
- URL-encoded: every field is `String` (`form.rs:135-138`); `into_form_pairs` (`form.rs:73-80`) pushes `(wire_name, value)` (`:161-162`). Check `self.body.<field>` (`&String` / `Option<String>`) in `into_parts` before the call.
- Multipart: text fields `String` (`form.rs:144`), JSON fields the registered type (`:143`), files `FormFile`/`Vec<FormFile>` (`:141-142`; rules must be rejected at definition time).
  JSON part today is `if let Ok(encoded) = serde_json::to_string(&value) { push }` (`form.rs:174-177`) - a serialize failure SILENTLY DROPS the part. For a constrained JSON form field, Rule 1 parity requires to_value -> check -> to_string of that Value, and a failure must become InvalidArgument, so `into_form_parts` (infallible `Vec<FormPart>`, `form.rs:66`) needs either a fallible sibling for constrained forms or the per-field JSON encoding hoisted into `into_parts`.
- Text: body is `String` (`body.rs:26`), sent via `self.body.into_bytes()` (`into_parts.rs:60-65`); check `&self.body` before.
- Committed generated multipart example: `schema/src/unfolded_circle_core_rest.rs:282-289, 341`. No committed urlencoded or text-body client exists; evidence for those is the generator source only.

## Q5 - Minimal runtime addition
None for encoding. The only runtime additions are the planned `validate` module items (`Plan`, `Violations`, ...) plus
`impl From<Violations> for InvalidArgument` (reason = aggregate `Display`; `InvalidArgument::new` takes `Cow<'static, str>`, `error.rs:110-123`).
`encode.rs`, `RequestBody`, `EndpointSpec`, and client paths stay unchanged. Optional: a generated/runtime helper
`validate::json_body(&T, &Plan) -> Result<String, InvalidArgument>` to keep emitted code short.
