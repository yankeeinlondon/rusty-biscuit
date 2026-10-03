#!/usr/bin/env python3
"""Throwaway generator for the lifecycle schema spike (2026-09-21-lifecycle-ergonomics).

Emits, under spike/schemas/:
  action.yaml                 `action` (exactly one verb key) and `bundle`
                              (verbs + when + no_error, >= 1 key) type tables
  action-collapsed.yaml       variant: generated verbs collapsed to `<string>: any`
  lifecycle.yaml              depth-5 unrolled stack grammar + loop block
  lifecycle-d{1..4}.yaml      reduced-depth variants for the growth curve
  lifecycle-collapsed.yaml    depth 5 over the collapsed action table
  lifecycle-collapsed-d3.yaml depth 3 over the collapsed action table

Usage: python3 gen-action-schema.py   (paths are absolute; run from anywhere)

Traps this generator works around (each is a finding in spike-results.md):
  T1  Darkmatter's standalone source-map projector cannot locate `- arm` lines
      that sit at the parent key's indentation (PyYAML's default style) and
      fails the whole file with "could not project SimplifiedSchema expression
      spans through YAML source". Sequences must be indented under their key.
  T2  The same projector rejects a plain scalar folded across physical lines
      (PyYAML's default `width`). Every type expression stays on one line.
  T3  `->` followed by an empty description is a grammar error.
  T4  The `kind: schema` envelope accepts only `kind` and `types` (no
      `description`, no exported `$schema`).
  T5  `Name[]@file` is rejected when `Name` is a union-typed named type, so a
      list item cannot be `bundle | conditional | bare-string`. The item is one
      merged mapping (verbs + when + then + else + no_error); Claudine's parser
      keeps enforcing exclusivity. A bare `- stop` string item is not typable.
"""
import copy
import re
from pathlib import Path

import yaml


class SchemaDumper(yaml.SafeDumper):
    """No `&id` aliases (T1 sibling), sequences indented under their key (T1)."""

    def ignore_aliases(self, data):
        return True

    def increase_indent(self, flow=False, indentless=False):
        return super().increase_indent(flow, False)


def dump(doc):
    # width=inf keeps every type expression on one physical line (T2)
    return yaml.dump(doc, Dumper=SchemaDumper, sort_keys=False, width=float("inf"))


REPO = Path("/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement")
OUT = REPO / "claudine/features/2026-09-21-lifecycle-ergonomics/spike/schemas"
CATALOG_RS = REPO / "darkmatter/lib/src/effects/catalog.rs"
FUNCS_YAML = REPO / "darkmatter/docs/schemas/expression-functions.yaml"

NO_ERR = "no_error: boolean"

# ---- hand-authored verbs (spec: Action Inventory) --------------------------
HAND = {}


def verb(name, short, long_params, desc):
    arms = []
    if short is not None:
        arms.append(f"{short} -> {desc}")
    if long_params is not None:
        arms.append("{ " + ", ".join(long_params + [NO_ERR]) + " }")
    HAND[name] = arms


verb("stop", "any", [], "stop the lifecycle stack (any event)")
verb("skip", "any", [], "skip the provider run (initialize only)")
verb("error", "string", ["reason: string"], "fail the run with a reason")
verb("proxy", "file", ["target: file(required)", "with: object"], "hand the run to another prompt")
verb("retry", "number(integer; min(0))",
     ["max_attempts: number(integer)", "delay: string", "backoff: enum(fixed, exponential)", "with: object"],
     "re-read and re-run the same document")
verb("resume", "string", ["message: string(required)", "max_attempts: number(integer)"],
     "send a follow-up message into the live session")
verb("defer", "string", ["delay: string(required)", "reason: string"], "planned: defer the run")
verb("break", "string", ["reason: string", "code: number(integer; min(0); max(255))"], "planned: leave the sequence")
verb("prep", "file", ["target: file(required)", "with: object"], "planned: prepare a target prompt")
for v in ["say", "speak", "effect", "message", "notify", "stderr", "info", "warn", "success", "stdout"]:
    verb(v, "string", ["text: string(required)"], f"{v}: emit text")
verb("shell", "string", ["command: string(required)", "on_error: string"], "run a shell command (not in initialize)")
HAND["set"] = ["{ <string>: any }(min-keys(1)) -> set runtime values (mapping only; no long form)"]


# ---- side-effect verbs from darkmatter/lib/src/effects/catalog.rs ----------
def side_effects():
    sigs = re.findall(r'signature: "([a-z_]+)\(([^)]*)\)"', CATALOG_RS.read_text())
    seen = {}
    for name, params in sigs:
        if name == "set":  # the runtime `set` directive owns this key
            continue
        plist = [p.strip() for p in params.split(",") if p.strip()]
        seen.setdefault(name, []).append(plist)
    out = {}
    for name, overloads in seen.items():
        overloads.sort(key=len)
        longest = overloads[-1]
        always = set(overloads[0])
        short = "any" if len(longest) == 1 else "any[]"
        long_params = [f"{p}: any{'(required)' if p in always else ''}" for p in longest]
        out[name] = [f"{short} -> side effect {name}({', '.join(longest)})",
                     "{ " + ", ".join(long_params + [NO_ERR]) + " }"]
    return out


# ---- expression functions from darkmatter/docs/schemas/expression-functions.yaml
TYPE_MAP = {"string": "string", "number": "number", "boolean": "boolean"}


def fn_type(p):
    return TYPE_MAP.get(p.get("type"), "any") + ("[]" if p.get("array") else "")


def expression_functions(collisions):
    data = yaml.safe_load(FUNCS_YAML.read_text())
    out = {}
    for f in data["functions"]:
        name = f["name"]
        if name in collisions:
            collisions[name] = "collides: skipped expression function"
            continue
        variadic = any(p.get("variadic") for o in f["overloads"] for p in o.get("parameters", []))
        maxn = max(len(o.get("parameters", [])) for o in f["overloads"])
        desc = f.get("description", "").replace("\n", " ").strip()
        short = "any" if maxn <= 1 else "any[]"
        arms = [f"{short} -> {desc}" if desc else short]  # T3
        if not variadic:
            for o in f["overloads"]:
                ps = o.get("parameters", [])
                long_params = [f"{p['name']}: {fn_type(p)}{'' if p.get('optional') else '(required)'}" for p in ps]
                arm = "{ " + ", ".join(long_params + [NO_ERR]) + " }"
                if arm not in arms:
                    arms.append(arm)
        out[name] = arms
    return out


def build_tables():
    table = dict(HAND)
    se = side_effects()
    collisions = {k: None for k in list(table) + list(se)}
    for k, v in se.items():
        if k in table:
            collisions[k] = "collides: side effect vs hand verb"
            continue
        table[k] = v
    ef = expression_functions(collisions)
    table.update(ef)
    return table, {k: v for k, v in collisions.items() if v}, len(HAND), len(se), len(ef)


def emit_action(path, table):
    action = copy.deepcopy(table)
    action["$constraints"] = {"min-keys": 1, "max-keys": 1}
    bundle = copy.deepcopy(table)
    bundle["when"] = "expression -> guard: the bundle runs only when true"
    bundle["no_error"] = "boolean -> suppress dispatch failures for every verb in the bundle"
    bundle["$constraints"] = {"min-keys": 1}
    path.write_text(dump({"kind": "schema", "types": {"action": action, "bundle": bundle}}))  # T4


def emit_lifecycle(path, depth, action_file, table):
    """Unrolled stack grammar; see T5 for why the item is one merged mapping."""
    types = {}

    def stack_union(n):
        return [f"item-{n}[]@this", f"bundle@./{action_file}"]

    types["item-0"] = f"bundle@./{action_file}"  # leaf of the unroll: no then/else
    types["stack-0"] = stack_union(0)
    for n in range(1, depth + 1):
        item = copy.deepcopy(table)
        item["when"] = "expression -> guard (the parser requires it when then/else are present)"
        item["then"] = stack_union(n - 1)
        item["else"] = stack_union(n - 1)
        item["no_error"] = "boolean"
        item["$constraints"] = {"min-keys": 1}
        types[f"item-{n}"] = item
        types[f"stack-{n}"] = stack_union(n)
    top = f"stack-{depth}@this"
    types["stack"] = top
    loop = {
        "while": "expression", "until": "expression",
        "action": ["string", "object", "any[]"], "actions": ["string", "object", "any[]"],
        "max": "number(integer; min(1))", "fail_fast": "boolean",
        "gate": top,
    }
    for k, v in HAND.items():
        loop[k] = copy.deepcopy(v)
    types["loop"] = loop
    path.write_text(dump({"kind": "schema", "types": types}))  # T4


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    table, collisions, n_hand, n_se, n_ef = build_tables()
    collapsed = {k: copy.deepcopy(table[k]) for k in HAND}
    collapsed["<string>"] = "any -> generated side-effect or expression-function verb"
    emit_action(OUT / "action.yaml", table)
    emit_action(OUT / "action-collapsed.yaml", collapsed)
    emit_lifecycle(OUT / "lifecycle.yaml", 5, "action.yaml", table)
    for d in (1, 2, 3, 4):
        emit_lifecycle(OUT / f"lifecycle-d{d}.yaml", d, "action.yaml", table)
    emit_lifecycle(OUT / "lifecycle-collapsed.yaml", 5, "action-collapsed.yaml", collapsed)
    emit_lifecycle(OUT / "lifecycle-collapsed-d3.yaml", 3, "action-collapsed.yaml", collapsed)
    print(f"hand={n_hand} side_effects={n_se} expression_functions={n_ef} total_keys={len(table)}")
    print("collisions:", collisions)
