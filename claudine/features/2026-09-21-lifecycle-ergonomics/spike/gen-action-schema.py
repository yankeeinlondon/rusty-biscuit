#!/usr/bin/env python3
"""Throwaway generator for the lifecycle schema spike.

Emits:
  schemas/action.yaml            -- `action` (exactly one verb key) and `bundle`
                                    (verbs + when + no_error, >=1 key) type tables
  schemas/lifecycle.yaml         -- depth-5 unrolled stack grammar + six events + loop
  schemas/lifecycle-d{N}.yaml    -- reduced-depth variants for the growth curve
  schemas/action-collapsed.yaml  -- variant: generated verbs collapsed to `<string>: any`
  schemas/lifecycle-collapsed.yaml

Usage: python3 gen-action-schema.py   (run from anywhere; paths are absolute)
"""
import re
import copy
import yaml

class NoAlias(yaml.SafeDumper):
    def ignore_aliases(self, data):
        return True
from pathlib import Path

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

verb("stop",  "any", [], "stop the lifecycle stack (any event)")
verb("skip",  "any", [], "skip the provider run (initialize only)")
verb("error", "string", ["reason: string"], "fail the run with a reason")
verb("proxy", "file", ["target: file(required)", "with: object"], "hand the run to another prompt")
verb("retry", "number(integer; min(0))",
     ["max_attempts: number(integer)", "delay: string", "backoff: enum(fixed, exponential)", "with: object"],
     "re-read and re-run the same document")
verb("resume", "string", ["message: string(required)", "max_attempts: number(integer)"],
     "send a follow-up message into the live session")
verb("defer", "string", ["delay: string(required)", "reason: string"], "planned: defer the run")
verb("break", "string", ["reason: string", "code: number(integer; min(0); max(255))"], "planned: leave the sequence")
verb("prep",  "file", ["target: file(required)", "with: object"], "planned: prepare a target prompt")
for v in ["say", "speak", "effect", "message", "notify", "stderr", "info", "warn", "success", "stdout"]:
    verb(v, "string", ["text: string(required)"], f"{v}: emit text")
verb("shell", "string", ["command: string(required)", "on_error: string"], "run a shell command (not in initialize)")
HAND["set"] = ["{ <string>: any }(min-keys(1)) -> set runtime values (mapping only; no long form)"]

# ---- side-effect verbs from darkmatter/lib/src/effects/catalog.rs ----------
def side_effects():
    sigs = re.findall(r'signature: "([a-z_]+)\(([^)]*)\)"', CATALOG_RS.read_text())
    seen = {}
    for name, params in sigs:
        if name == "set":            # the runtime `set` directive owns this key
            continue
        plist = [p.strip() for p in params.split(",") if p.strip()]
        seen.setdefault(name, []).append(plist)
    out = {}
    for name, overloads in seen.items():
        overloads.sort(key=len)
        longest = overloads[-1]
        always = set(overloads[0])
        maxn = len(longest)
        short = "any" if maxn == 1 else "any[]"
        long_params = [f"{p}: any{'(required)' if p in always else ''}" for p in longest]
        out[name] = [f"{short} -> side effect {name}({', '.join(longest)})",
                     "{ " + ", ".join(long_params + [NO_ERR]) + " }"]
    return out

# ---- expression functions from darkmatter/docs/schemas/expression-functions.yaml
TYPE_MAP = {"string": "string", "number": "number", "boolean": "boolean"}
def fn_type(p):
    base = TYPE_MAP.get(p.get("type"), "any")
    return base + ("[]" if p.get("array") else "")

def expression_functions(collisions):
    data = yaml.safe_load(FUNCS_YAML.read_text())
    out = {}
    for f in data["functions"]:
        name = f["name"]
        if name in collisions:
            collisions[name] = "collides: skipped expression function"
            continue
        arms = []
        variadic = any(p.get("variadic") for o in f["overloads"] for p in o.get("parameters", []))
        maxn = max(len(o.get("parameters", [])) for o in f["overloads"])
        desc = f.get("description", "").replace("\n", " ").strip()
        short = "any" if maxn <= 1 else "any[]"
        # TRAP: `->` with an empty description is a grammar error; omit the arrow instead
        arms.append(f"{short} -> {desc}" if desc else short)
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

def emit_action(path, table, collapsed=False):
    if collapsed:
        # hand verbs typed; every generated verb collapsed to a catch-all
        keep = {k: table[k] for k in HAND}
        keep["<string>"] = "any -> generated side-effect or expression-function verb"
        table = keep
    action = copy.deepcopy(table)
    action["$constraints"] = {"min-keys": 1, "max-keys": 1}
    bundle = copy.deepcopy(table)
    bundle["when"] = "expression -> guard: the bundle runs only when true"
    bundle["no_error"] = "boolean -> suppress dispatch failures for every verb in the bundle"
    bundle["$constraints"] = {"min-keys": 1}
    # TRAP: the tagged envelope accepts only `kind` and `types` (no description, no $schema)
    doc = {"kind": "schema", "types": {"action": action, "bundle": bundle}}
    path.write_text(yaml.dump(doc, Dumper=NoAlias, sort_keys=False, width=200))

def emit_lifecycle(path, depth, action_file):
    types = {}
    # stack-0: no conditional arm at all
    types["leaf"] = [f"bundle[]@./{action_file}", f"bundle@./{action_file}"]
    types["stack-0"] = ["leaf@this"]
    for n in range(1, depth + 1):
        inner = f"stack-{n-1}@this"
        types[f"cond-{n}"] = {
            "when": "expression(required) -> condition",
            "then": f"{inner}",
            "else": f"{inner}",
        }
        types[f"item-{n}"] = ["enum(stop, skip) -> bare zero-argument verb",
                              f"bundle@./{action_file}", f"cond-{n}@this"]
        types[f"stack-{n}"] = [f"item-{n}[]@this", f"bundle@./{action_file}"]
    top = f"stack-{depth}@this"
    types["stack"] = [top]
    loop = {
        "while": "expression", "until": "expression",
        "action": ["string", "object", "any[]"], "actions": ["string", "object", "any[]"],
        "max": "number(integer; min(1))", "fail_fast": "boolean",
        "gate": top,
    }
    # loose verb keys at the loop root: reuse the hand verb table
    for k, v in HAND.items():
        loop[k] = v
    types["loop"] = loop
    # TRAP: a `kind: schema` library cannot also export `$schema`; the trigger payload
    # (claudine.yaml) or a document's inline `$schema:` mapping wires the events.
    doc = {"kind": "schema", "types": types}
    path.write_text(yaml.dump(doc, Dumper=NoAlias, sort_keys=False, width=200))

if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    table, collisions, n_hand, n_se, n_ef = build_tables()
    emit_action(OUT / "action.yaml", table)
    emit_action(OUT / "action-collapsed.yaml", table, collapsed=True)
    emit_lifecycle(OUT / "lifecycle.yaml", 5, "action.yaml")
    for d in (1, 2, 3, 4):
        emit_lifecycle(OUT / f"lifecycle-d{d}.yaml", d, "action.yaml")
    emit_lifecycle(OUT / "lifecycle-collapsed.yaml", 5, "action-collapsed.yaml")
    print(f"hand={n_hand} side_effects={n_se} expression_functions={n_ef} total_keys={len(table)}")
    print("collisions:", collisions)
