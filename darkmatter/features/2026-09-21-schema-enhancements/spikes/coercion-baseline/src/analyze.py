#!/usr/bin/env python3
"""THROWAWAY SPIKE: turns baseline.json into ledger-draft.md plus a stats JSON.

Usage: python3 analyze.py <baseline.json> <out dir>
"""
import json
import sys
from collections import Counter, defaultdict, OrderedDict

baseline_path, out_dir = sys.argv[1], sys.argv[2]
data = json.load(open(baseline_path))
records = data["records"]

PRESERVED_PREFIXES = ("preserved",)


def cause(diff: str) -> str:
    d = diff.replace("missing-signature | ", "")
    if d.startswith("preserved"):
        return None
    if d.startswith("null: propagates"):
        return "null: propagates today (returns null) -> type error"
    if d.startswith("null: consumed"):
        return "null: consumed as a value today (e.g. 0, false) -> type error"
    if d.startswith("null: reaches"):
        return "null: treated as 'absent' today -> type error"
    if d.startswith("null: rejected today"):
        return "null: rejected today -> accepted"
    if "via conversion: number->text" in d or "via conversion: boolean->text" in d:
        return "newly accepted: number/boolean -> text"
    if "via conversion: text->number" in d:
        return "newly accepted: numeric text -> number"
    if d.startswith("changed result (conversion boolean->text"):
        return "changed result: boolean now read as its text (\"true\")"
    if d.startswith("changed result (conversion text->number"):
        return "changed result: lenient number grammar (\" 4 \", \"1_000\")"
    if d.startswith("changed result"):
        return "changed result: other"
    if d == "newly rejected: boolean->number":
        return "newly rejected: boolean -> number"
    if d.startswith("newly rejected: text the handler parsed"):
        return "newly rejected: unparseable text the handler turned into 0/fallback"
    if d.startswith("newly rejected: container/null to text"):
        return "newly rejected: list/object to a text parameter (handler stringified it)"
    if d.startswith("newly rejected: expected object"):
        return "newly rejected: non-object to an object parameter (handler tolerated it)"
    if d.startswith("newly rejected: expected number"):
        return "newly rejected: list/object to a number parameter (handler fell back to 0)"
    if d.startswith("newly rejected: arity"):
        return "newly rejected: arity (draft has fewer parameters)"
    if d.startswith("newly rejected"):
        return "newly rejected: other"
    if d.startswith("draft signature wider"):
        return "draft signature wider than code (engine binds, handler still type-checks)"
    if d.startswith("type error today -> content error"):
        return "error class: type error today -> converted, then content/domain error"
    if d.startswith("changed outcome class"):
        return "changed outcome class"
    return "other: " + d


per_fn = OrderedDict()
for r in records:
    fn = r["function"]
    entry = per_fn.setdefault(fn, {"source": r["signature_source"], "total": 0, "causes": defaultdict(list)})
    entry["total"] += 1
    c = cause(r["diff"])
    if c:
        entry["causes"][c].append(f"{r['param']}={r['sample']}")

cause_records = Counter()
cause_functions = defaultdict(set)
for fn, e in per_fn.items():
    for c, samples in e["causes"].items():
        cause_records[c] += len(samples)
        cause_functions[c].add(fn)

changed = [fn for fn, e in per_fn.items() if e["causes"]]
preserved = [fn for fn, e in per_fn.items() if not e["causes"]]
missing = [fn for fn, e in per_fn.items() if e["source"].startswith("missing")]

# Null policy stats (today, per function, over every position probed with null)
null_today = defaultdict(set)
for r in records:
    if r["sample"] != "null":
        continue
    t = r["today"]
    if t["kind"] == "ok" and t.get("value") is None:
        null_today["propagates (returns null)"].add(r["function"])
    elif t["kind"] == "ok":
        null_today["consumed as a value"].add(r["function"])
    elif t["kind"] in ("arg-error", "arity", "value-invalid"):
        null_today["rejected"].add(r["function"])
    else:
        null_today["reaches domain/io (treated as absent)"].add(r["function"])

null_pred_accepts = sorted({r["function"] for r in records if r["sample"] == "null" and r["predicted"]["kind"] not in ("type-error", "arity", "ambiguous")})
needs_null = sorted({r["function"] for r in records if r["sample"] == "null" and r["diff"].replace("missing-signature | ", "").startswith("null:") and "accepted" not in r["diff"]})

stats = {
    "records": len(records),
    "functions": len(per_fn),
    "functions_fully_preserved": len(preserved),
    "functions_changed": len(changed),
    "missing_signature": missing,
    "diff_classes": Counter(r["diff"] for r in records).most_common(),
    "cause_records": cause_records.most_common(),
    "cause_functions": {c: sorted(v) for c, v in cause_functions.items()},
    "null_today": {k: sorted(v) for k, v in null_today.items()},
    "null_accepted_by_draft": null_pred_accepts,
    "functions_needing_null_to_keep_today": needs_null,
}
json.dump(stats, open(f"{out_dir}/stats.json", "w"), indent=2)

# ---------- ledger-draft.md ----------
lines = []
w = lines.append
w("# Behavior-change ledger (draft, machine-generated by the coercion-baseline spike)")
w("")
w("> Throwaway spike output, 2026-09-26. Today = the current runtime, called through the")
w("> normal evaluator. Predicted = the spec's coercion rules applied to the **draft** catalog")
w("> signatures (`claudine/docs/schemas/partials/functions.yaml`), then the unchanged handler")
w("> run on the bound values. It is a draft for the one-pass migration's ledger, not the ledger.")
w("> The 14 functions missing from the draft were predicted from their old-catalog types and are")
w("> marked `missing signature`.")
w("")
w("## Summary")
w("")
w(f"- Functions probed: **{len(per_fn)}** (all registered, including `and`/`or`)")
w(f"- Calls recorded: **{len(records)}** (25 sample inputs x each parameter position, other parameters valid)")
w(f"- Functions whose recorded behavior is fully **preserved**: **{len(preserved)}**")
w(f"- Functions with at least one **change**: **{len(changed)}** ({len([f for f in changed if f not in missing])} with a draft signature, {len([f for f in changed if f in missing])} missing from the draft)")
w("")
risky, risky_nonnull, benign = set(), set(), set()
for r in records:
    x = r["diff"].replace("missing-signature | ", "")
    if x.startswith("null:") and "accepted" not in x:
        risky.add(r["function"])
    elif x.startswith("newly rejected") or x.startswith("changed"):
        risky.add(r["function"]); risky_nonnull.add(r["function"])
    elif "accepted" in x or "content error" in x or "wider" in x:
        benign.add(r["function"])
w("### Risk split")
w("")
w(f"- **Risky** (a call that succeeds today fails or returns something else): **{len(risky)}** functions;")
w(f"  **{len(risky) - len(risky_nonnull)}** of them only because of `null`.")
w(f"- Risky for a non-null reason: **{len(risky_nonnull)}**: {', '.join(f'`{f}`' for f in sorted(risky_nonnull))}")
w(f"- **Benign only** (errors become successes, or one error becomes another): **{len(benign - risky)}** functions")
w("")
w("### Changes by cause")
w("")
w("| Cause | Calls | Functions |")
w("|---|---:|---:|")
for c, n in cause_records.most_common():
    w(f"| {c} | {n} | {len(cause_functions[c])} |")
w("")
w("### Null handling today")
w("")
for k, v in null_today.items():
    w(f"- **{k}**: {len(v)} functions")
w(f"- Functions that would need `| null` on some parameter to keep today's null result: **{len(needs_null)}**")
w("")
w("## Per function")
w("")
w("Legend: `param=sample` lists the probes that changed. `preserved` means every probe matched")
w("(same value, same rejection, or the same content/domain/IO outcome).")
w("")
for fn, e in sorted(per_fn.items()):
    sig = data["signatures"][fn]
    status = "changed" if e["causes"] else "preserved"
    tag = " (missing signature)" if fn in missing else ""
    w(f"### `{fn}` — {status}{tag}")
    w("")
    w(f"- Draft: `{' || '.join(sig['draft'])[:300]}`" if not fn in missing else f"- Old catalog: `{' || '.join(sig['old'])}`")
    if sig["notes"] and fn not in missing:
        w(f"- Grammar notes: {'; '.join(sig['notes'])}")
    for c, samples in sorted(e["causes"].items(), key=lambda kv: -len(kv[1])):
        shown = ", ".join(samples[:8]) + (f", ... (+{len(samples) - 8})" if len(samples) > 8 else "")
        w(f"- **{c}** ({len(samples)}): {shown}")
    w("")

open(f"{out_dir}/ledger-draft.md", "w").write("\n".join(lines) + "\n")
print(json.dumps({k: stats[k] for k in ("records", "functions", "functions_fully_preserved", "functions_changed")}))
print("\n".join(f"{n:5d} {c} [{len(cause_functions[c])} fns]" for c, n in cause_records.most_common()))
print({k: len(v) for k, v in null_today.items()}, "needs_null", len(needs_null))
