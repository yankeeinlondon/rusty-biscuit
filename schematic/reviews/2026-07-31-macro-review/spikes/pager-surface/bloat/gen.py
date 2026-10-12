#!/usr/bin/env python3
"""Synthetic expansion for check (h).

gen.py <n> <base|paged|paged1> <outdir> <spike_dir>

Writes a standalone crate with <n> endpoints. Both modes emit one item
struct per endpoint and decode it (today's unpaginated cost). `paged` adds
what the generator would emit for a pager (unit struct impls plus a one-line
constructor), cycling through six strategy templates, and a binary that
drives every pager so each `Pager<E>`/`Pages<E>` is monomorphized.
`paged1` emits the same code as `paged` but drives only one pager: the
control that links the same HTTP stack, so `paged` minus `paged1` is the
per-endpoint monomorphization cost.
"""
import os
import sys

n, mode, out, spike = int(sys.argv[1]), sys.argv[2], sys.argv[3], os.path.abspath(sys.argv[4])
os.makedirs(f"{out}/src", exist_ok=True)

open(f"{out}/Cargo.toml", "w").write(f"""[package]
name = "bloat-{mode}"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
pager-surface = {{ path = "{spike}" }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
tokio = {{ version = "1", features = ["rt", "time", "net"] }}
""")

TEMPLATES = [
    # page number + size
    """impl Paginated for Ep{i} {{
    type Kind = PageNumber;
    const STRATEGY: &'static PageNumberSpec = &PageNumberSpec {{ page: Input::Query("page"), base: 1, more: More::LinkRel("next") }};
}}
impl PageSizeKnob for Ep{i} {{
    const PAGE_SIZE: SizeSpec = SizeSpec {{ param: Input::Query("per_page"), min: 1, max: Some(100) }};
}}""",
    # opaque cursor
    """impl Paginated for Ep{i} {{
    type Kind = Cursor;
    const STRATEGY: &'static CursorSpec = &CursorSpec {{
        forward: CursorLeg {{ param: Input::Query("after"), from: ValueAt::Body(&["last_id"]) }},
        more: Some(More::Flag(ValueAt::Body(&["has_more"]))),
    }};
}}""",
    # resumable bidirectional cursor + size
    """impl Paginated for Ep{i} {{
    type Kind = Cursor;
    const STRATEGY: &'static CursorSpec = &CursorSpec {{
        forward: CursorLeg {{ param: Input::Query("after_id"), from: ValueAt::Body(&["last_id"]) }},
        more: Some(More::Flag(ValueAt::Body(&["has_more"]))),
    }};
}}
impl ResumeAfter for Ep{i} {{}}
impl HasPrev for Ep{i} {{
    const PREV: &'static CursorLeg = &CursorLeg {{ param: Input::Query("before_id"), from: ValueAt::Body(&["first_id"]) }};
}}
impl PageSizeKnob for Ep{i} {{
    const PAGE_SIZE: SizeSpec = SizeSpec {{ param: Input::Query("limit"), min: 1, max: Some(1000) }};
}}""",
    # next link in body + size
    """impl Paginated for Ep{i} {{
    type Kind = NextLink;
    const STRATEGY: &'static LinkSpec = &LinkSpec {{ next: LinkFrom::Body(&["next"]) }};
}}
impl PageSizeKnob for Ep{i} {{
    const PAGE_SIZE: SizeSpec = SizeSpec {{ param: Input::Query("pagelen"), min: 1, max: Some(100) }};
}}""",
    # Link header + prev
    """impl Paginated for Ep{i} {{
    type Kind = NextLink;
    const STRATEGY: &'static LinkSpec = &LinkSpec {{ next: LinkFrom::Header("next") }};
}}
impl HasPrev for Ep{i} {{
    const PREV: &'static LinkFrom = &LinkFrom::Header("prev");
}}""",
    # scroll session
    """impl Paginated for Ep{i} {{
    type Kind = Scroll;
    const STRATEGY: &'static ScrollSpec = &ScrollSpec {{
        open_keep_alive: Input::Query("scroll"),
        default_keep_alive: "1m",
        follow_path: "/_search/scroll",
        handle_from: ValueAt::Body(&["_scroll_id"]),
        handle_param: Input::Body("scroll_id"),
        keep_alive_param: Input::Body("scroll"),
        close_path: "/_search/scroll",
    }};
}}
impl PageSizeKnob for Ep{i} {{
    const PAGE_SIZE: SizeSpec = SizeSpec {{ param: Input::Body("size"), min: 1, max: Some(10_000) }};
}}
impl KeepAliveKnob for Ep{i} {{}}""",
]

lib = ["#![allow(dead_code)]", "use serde::Deserialize;"]
if mode in ("paged", "paged1"):
    lib.append("use pager_surface::{Client, Pager, client::Method, spec::*};")
for i in range(n):
    lib.append(f"""
#[derive(Debug, Deserialize)]
pub struct Item{i} {{
    pub id: u64,
    pub name: String,
    pub tags: Vec<String>,
}}
/// Today's unpaginated list decode, present in both modes.
pub fn decode_{i}(body: &[u8]) -> serde_json::Result<Vec<Item{i}>> {{
    serde_json::from_slice(body)
}}""")
    if mode in ("paged", "paged1"):
        lib.append(f"""// ---- generated pager for Ep{i} ----
pub struct Ep{i};
impl Endpoint for Ep{i} {{
    type Item = Item{i};
    const SPEC: &'static EndpointSpec = &EndpointSpec {{ method: Method::Get, path: "/ep/{i}", items: &["data"], total: None }};
}}
{TEMPLATES[i % len(TEMPLATES)].format(i=i)}
pub fn ep{i}(client: &Client) -> Pager<Ep{i}> {{
    Pager::new(client)
}}
// ---- end Ep{i} ----""")
open(f"{out}/src/lib.rs", "w").write("\n".join(lib) + "\n")

main = ["fn main() {", "    let armed = std::env::args().count() > 100;", "    let body = br#\"[]\"#;"]
for i in range(n):
    main.append(f"    if armed {{ let _ = bloat_{mode}::decode_{i}(body); }}")
if mode in ("paged", "paged1"):
    main.append("    if armed {")
    main.append("        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();")
    main.append("        let client = pager_surface::Client::new(\"http://127.0.0.1:1\");")
    main.append("        rt.block_on(async {")
    crate = "bloat_" + mode
    for i in range(n if mode == "paged" else 1):
        main.append(f"            {{ let mut p = {crate}::ep{i}(&client).pages(); let _ = p.next().await; }}")
    main.append("        });")
    main.append("    }")
main.append("}")
open(f"{out}/src/main.rs", "w").write("\n".join(main) + "\n")
