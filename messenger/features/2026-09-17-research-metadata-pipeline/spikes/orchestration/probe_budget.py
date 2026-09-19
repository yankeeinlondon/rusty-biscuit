"""Phase 1 budget-spike probes against a Claudine binary, using a fake provider.

Offline macOS/Linux fixture. It never contacts a model service: the only
"provider" is a fake `goose` executable written into a temporary directory.

Usage (from the repository root):

    CLAUDINE_BIN=/path/to/claudine SOURCE_REVISION=<sha or 'unverified'> \
        python3 messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/probe_budget.py [--out FILE]

Sections:
- native_*   exercise the given Claudine binary unchanged.
- ledger_*   exercise a PROTOTYPE of the proposed persistent budget ledger
             protocol (Python, this file). They are not Claudine behavior.
"""

import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parent
MARK_PRIOR = "PREVIOUS_PROSE_MARKER"


# --------------------------------------------------------------------------
# Fake provider (runs when this file is invoked as `goose`)
# --------------------------------------------------------------------------

def _files_with_marker(start, limit=400):
    hits, seen = [], 0
    for directory, _dirs, files in os.walk(start):
        for name in files:
            seen += 1
            if seen > limit:
                return hits
            path = os.path.join(directory, name)
            try:
                with open(path, encoding="utf-8", errors="ignore") as handle:
                    if MARK_PRIOR in handle.read(65536):
                        hits.append(os.path.relpath(path, start))
            except OSError:
                pass
    return hits


def _instruction_files(cwd):
    found, current = [], Path(cwd)
    for directory in [current, *current.parents]:
        for name in ("CLAUDE.md", "AGENTS.md", "GEMINI.md"):
            if (directory / name).exists():
                found.append(str(directory / name))
    return found


def fake_provider():
    args = sys.argv[1:]
    if args == ["--version"]:
        print("goose 1.0.0-fixture")
        return
    text = " ".join(args)
    cwd = os.getcwd()
    record = {
        "pid": os.getpid(),
        "pgid": os.getpgid(0),
        "sid": os.getsid(0),
        "cwd": cwd,
        "argv_flags": [a for a in args if a.startswith("-")],
        "prompt_has_prior_marker": MARK_PRIOR in text,
        "prompt_mentions_document_path": ".md" in text,
        "instruction_files_visible": _instruction_files(cwd),
        "prior_marker_files_under_cwd": _files_with_marker(cwd),
        "env_keys": sorted(k for k in os.environ if k.isupper()),
        "seq": next((tok.split("=", 1)[1] for tok in text.split() if tok.startswith("SEQ=")), None),
        "tag": next((tok for tok in ("DISCOVERY", "INLINE", "TREE", "IGNORE_TERM",
                                     "ORPHAN", "RETRY", "STEP_ONE", "STEP_TWO")
                     if tok in text), None),
        "t": time.time(),
    }
    records = os.environ["PROBE_RECORDS"]
    if "INLINE" in text:
        # Inline guardrails tell the agent to edit the named document directly.
        doc = next((tok.strip("`*()[]<>,") for tok in text.split() if tok.strip("`*()[]<>,").endswith(".md")), None)
        record["inline_document_token"] = doc
        if doc and os.path.exists(doc):
            body = Path(doc).read_text(encoding="utf-8")
            record["inline_document_readable_with_prior"] = MARK_PRIOR in body
            head, _sep, _rest = body.partition("\n---\n")
            Path(doc).write_text(head + "\n---\nRefreshed body.\n", encoding="utf-8")
    if "TREE" in text:
        same = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"])
        detached = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"],
                                    start_new_session=True)
        record["grandchild_same_group"] = same.pid
        record["grandchild_new_session"] = detached.pid
    if "IGNORE_TERM" in text:
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
    with open(records, "a", encoding="utf-8") as output:
        output.write(json.dumps(record) + "\n")
    if "RETRY" in text or "STEP_TWO" in text:
        sys.exit(7)
    if any(tag in text for tag in ("TREE", "IGNORE_TERM", "ORPHAN")):
        time.sleep(30)
    print("Fixture result.")


# --------------------------------------------------------------------------
# Native probes
# --------------------------------------------------------------------------

def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    # A zombie still answers kill(0); ask ps for its state.
    state = subprocess.run(["ps", "-o", "stat=", "-p", str(pid)], capture_output=True, text=True).stdout.strip()
    return bool(state) and not state.startswith("Z")


def reap(pids):
    for pid in pids:
        if pid and alive(pid):
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass


def read_records(path):
    return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []


def invoke(binary, env, cwd, command, timeout=60):
    start = time.monotonic()
    result = subprocess.run([binary, *command], cwd=cwd, env=env, capture_output=True,
                            text=True, timeout=timeout)
    return {"command": command, "code": result.returncode,
            "elapsed_seconds": round(time.monotonic() - start, 2),
            "stderr_tail": result.stderr.strip().splitlines()[-3:]}


def doc(path, frontmatter, body):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("---\n" + frontmatter + "---\n" + body + "\n", encoding="utf-8")


def native_probes(binary, workspace, env):
    out = {}
    records = Path(env["PROBE_RECORDS"])
    repo = workspace / "repo"
    (repo / "research").mkdir(parents=True)
    subprocess.run(["git", "init", "-q", str(repo)], check=True)
    (repo / "CLAUDE.md").write_text("Repository instructions fixture.\n")
    (repo / "AGENTS.md").write_text("Agent instructions fixture.\n")
    doc(repo / "research" / "prior.md", "title: prior\n", MARK_PRIOR + " accepted prose.")
    doc(repo / "work" / "discovery.md", "agent: goose\n", "DISCOVERY provider.example questions schema")

    # 1a. Discovery launched from inside a repository.
    before = len(read_records(records))
    out["isolation_repo_launch"] = invoke(binary, env, repo / "work", ["compose", "--goose", "discovery.md"])
    out["isolation_repo_launch"]["worker"] = read_records(records)[before:]

    # 1b. Discovery launched from a scratch directory outside any repository.
    scratch = workspace / "scratch"
    doc(scratch / "discovery.md", "agent: goose\n", "DISCOVERY provider.example questions schema")
    before = len(read_records(records))
    out["isolation_scratch_launch"] = invoke(binary, env, scratch, ["compose", "--goose", "discovery.md"])
    out["isolation_scratch_launch"]["worker"] = read_records(records)[before:]

    # 1c. Same scratch launch with --repo (provider overlay mode).
    before = len(read_records(records))
    out["isolation_scratch_repo_flag"] = invoke(binary, env, scratch, ["compose", "--goose", "--repo", "discovery.md"])
    out["isolation_scratch_repo_flag"]["worker"] = read_records(records)[before:]

    # 2. Inline-compose refresh of an existing research document.
    doc(repo / "research" / "platform.md", "agent: goose\nprompt: INLINE refresh the research\n",
        MARK_PRIOR + " prior accepted research prose.")
    before = len(read_records(records))
    out["inline_compose_refresh"] = invoke(binary, env, repo / "research", ["inline-compose", "--goose", "platform.md"])
    out["inline_compose_refresh"]["worker"] = read_records(records)[before:]

    # 3a. Timeout with a same-group and a new-session grandchild.
    doc(scratch / "tree.md", "agent: goose\n", "TREE stalled source check")
    before = len(read_records(records))
    out["tree_cancel"] = invoke(binary, env, scratch, ["compose", "--goose", "--timeout", "1s", "tree.md"])
    worker = read_records(records)[before:]
    time.sleep(0.5)
    if worker:
        w = worker[0]
        out["tree_cancel"]["survivors"] = {
            "worker": alive(w["pid"]),
            "grandchild_same_group": alive(w["grandchild_same_group"]),
            "grandchild_new_session": alive(w["grandchild_new_session"]),
        }
        reap([w["pid"], w["grandchild_same_group"], w["grandchild_new_session"]])

    # 3b. SIGTERM-ignoring worker: grace period is wall time the budget must charge.
    doc(scratch / "ignore.md", "agent: goose\n", "IGNORE_TERM stalled source check")
    grace_env = dict(env, CLAUDINE_KILL_GRACE="2s")
    before = len(read_records(records))
    out["term_ignored_grace"] = invoke(binary, grace_env, scratch,
                                       ["compose", "--goose", "--timeout", "1s", "ignore.md"])
    worker = read_records(records)[before:]
    if worker:
        out["term_ignored_grace"]["worker_alive_after"] = alive(worker[0]["pid"])
        reap([worker[0]["pid"]])

    # 3c. Wrapper crash: SIGKILL Claudine while its worker runs.
    doc(scratch / "orphan.md", "agent: goose\n", "ORPHAN long source check")
    before = len(read_records(records))
    proc = subprocess.Popen([binary, "compose", "--goose", "orphan.md"], cwd=scratch, env=env,
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline and len(read_records(records)) == before:
        time.sleep(0.1)
    proc.send_signal(signal.SIGKILL)
    proc.wait()
    time.sleep(0.5)
    worker = read_records(records)[before:]
    out["wrapper_sigkill"] = {"worker_launched": bool(worker),
                              "worker_alive_after_wrapper_sigkill": bool(worker) and alive(worker[0]["pid"])}
    if worker:
        reap([worker[0]["pid"]])

    # 4a. Lifecycle retry with delay: several launches inside one command.
    doc(scratch / "retry.md",
        "agent: goose\nfailure:\n  stack:\n    - action:\n        action: retry\n        max_attempts: 2\n"
        "        delay: \"1s\"\n",
        "RETRY reconciliation")
    before = len(read_records(records))
    out["retry_with_delay"] = invoke(binary, env, scratch, ["compose", "--goose", "--timeout", "5s", "retry.md"])
    launches = read_records(records)[before:]
    out["retry_with_delay"]["launches"] = len(launches)
    out["retry_with_delay"]["launch_gaps_seconds"] = [round(b["t"] - a["t"], 2) for a, b in zip(launches, launches[1:])]

    # 4b. Restart: a failed sequence re-run starts from step one with a new id.
    doc(scratch / "one.md", "agent: goose\n", "STEP_ONE SEQ={{ sequence_id }}")
    doc(scratch / "two.md", "agent: goose\n", "STEP_TWO SEQ={{ sequence_id }}")
    (scratch / "seq.md").write_text("---\nsequence:\n  - name: one\n    prompt: one.md\n"
                                    "  - name: two\n    prompt: two.md\n---\n")
    runs = []
    for _ in range(2):
        before = len(read_records(records))
        result = invoke(binary, env, scratch, ["sequence", "--goose", "seq.md"])
        result["steps"] = [(r["tag"], r["seq"]) for r in read_records(records)[before:]]
        runs.append(result)
    out["sequence_restart"] = runs

    # 4c. What Claudine left on disk under the private home.
    home = Path(env["HOME"])
    out["private_home_files_after_runs"] = sorted(
        str(p.relative_to(home)) for p in home.rglob("*") if p.is_file())[:60]
    return out


# --------------------------------------------------------------------------
# Prototype of the proposed ledger protocol (NOT Claudine behavior)
# --------------------------------------------------------------------------

HEARTBEAT = 0.5


def ledger_write(path, ledger):
    tmp = path.with_suffix(".tmp")
    with open(tmp, "w", encoding="utf-8") as handle:
        json.dump(ledger, handle, indent=1)
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(tmp, path)
    if os.name != "nt":
        fd = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)


def ledger_new(path, invocations, seconds):
    ledger_write(path, {"schema": 1, "platform": "fixture", "state": "active",
                        "limits": {"invocations": invocations, "active_seconds": seconds},
                        "grants": [], "used": {"invocations": 0, "active_seconds": 0.0},
                        "segment": {"opened_at": time.time(), "heartbeat_at": time.time()},
                        "in_flight": None, "events": []})


def ledger_load(path):
    return json.loads(path.read_text())


def _limit(ledger, key):
    return ledger["limits"][key] + sum(g.get(key, 0) for g in ledger["grants"])


def _fold_segment(ledger, now):
    # Charge wall time since the segment's last fold: waits, backoff, and
    # orchestration are active time, not only child runtime.
    seg = ledger["segment"]
    if seg:
        ledger["used"]["active_seconds"] += max(0.0, now - seg["heartbeat_at"])
        seg["heartbeat_at"] = now


def ledger_recover(path):
    """Crash recovery: charge through last heartbeat + one interval, no refund."""
    ledger = ledger_load(path)
    if ledger["state"] == "active" and ledger["segment"]:
        seg = ledger["segment"]
        ledger["used"]["active_seconds"] += HEARTBEAT  # conservative upper bound of the unseen tail
        flight = ledger["in_flight"]
        if flight:
            try:
                os.killpg(flight["pgid"], signal.SIGKILL)
                orphan = "killed"
            except ProcessLookupError:
                orphan = "gone"
            ledger["events"].append({"kind": "crash_recovered", "in_flight_invocation": flight["n"],
                                     "orphan_group": orphan, "last_heartbeat": seg["heartbeat_at"]})
            ledger["in_flight"] = None
        ledger["state"] = "interrupted"
        ledger["segment"] = None
        ledger_write(path, ledger)
    return ledger


def ledger_transition(path, state, reason):
    ledger = ledger_load(path)
    now = time.time()
    if state in ("suspended", "stopped"):
        _fold_segment(ledger, now)
        ledger["segment"] = None
    elif state == "active":
        ledger["segment"] = {"opened_at": now, "heartbeat_at": now}
    ledger["state"] = state
    ledger["events"].append({"kind": state, "reason": reason, "at": now})
    ledger_write(path, ledger)


def ledger_grant(path, operator, invocations=0, seconds=0):
    ledger = ledger_load(path)
    ledger["grants"].append({"operator": operator, "invocations": invocations,
                             "active_seconds": seconds, "at": time.time()})
    ledger_write(path, ledger)


def ledger_launch(path, argv, env):
    """Admission: charge the invocation BEFORE spawn; settle time after."""
    ledger = ledger_load(path)
    if ledger["state"] != "active":
        return {"admitted": False, "reason": "state:" + ledger["state"]}
    _fold_segment(ledger, time.time())
    used, limits = ledger["used"], {k: _limit(ledger, k) for k in ("invocations", "active_seconds")}
    if used["invocations"] >= limits["invocations"] or used["active_seconds"] >= limits["active_seconds"]:
        ledger["state"] = "exhausted"
        ledger["events"].append({"kind": "exhausted", "used": dict(used)})
        ledger_write(path, ledger)
        return {"admitted": False, "reason": "exhausted"}
    used["invocations"] += 1
    remaining = limits["active_seconds"] - used["active_seconds"]
    ledger["in_flight"] = {"n": used["invocations"], "pgid": None, "admitted_at": time.time()}
    ledger_write(path, ledger)  # durable charge before any child exists
    child = subprocess.Popen(argv, env=env, start_new_session=True)
    ledger["in_flight"]["pgid"] = child.pid
    ledger_write(path, ledger)
    deadline = time.time() + remaining
    while child.poll() is None:
        time.sleep(0.05)
        now = time.time()
        if now - ledger["segment"]["heartbeat_at"] >= HEARTBEAT:
            _fold_segment(ledger, now)
            ledger_write(path, ledger)
        if now >= deadline:
            os.killpg(child.pid, signal.SIGKILL)
    _fold_segment(ledger, time.time())
    ledger["in_flight"] = None
    ledger_write(path, ledger)
    return {"admitted": True, "code": child.returncode, "remaining_at_launch": round(remaining, 2)}


def ledger_probes(workspace):
    out = {}
    sleeper = lambda secs, code=0: [sys.executable, "-c", f"import time,sys; time.sleep({secs}); sys.exit({code})"]
    env = {"PATH": os.environ.get("PATH", "")}
    path = workspace / "ledger.json"

    # Retry and automatic backoff share one allowance; the wait is charged.
    ledger_new(path, invocations=5, seconds=30)
    first = ledger_launch(path, sleeper(0.2, 7), env)
    time.sleep(1.0)  # automatic backoff before the retry: active time
    retry = ledger_launch(path, sleeper(0.2), env)
    after = ledger_load(path)["used"]
    out["retry_and_backoff"] = {"codes": [first["code"], retry["code"]], "used": after,
                                "backoff_charged": after["active_seconds"] >= 1.4}

    # Explicit suspension for human approval: wall time is not charged.
    before = ledger_load(path)["used"]["active_seconds"]
    ledger_transition(path, "suspended", "awaiting human approval")
    time.sleep(1.0)
    refused = ledger_launch(path, sleeper(0), env)
    ledger_transition(path, "active", "operator resumed")
    resumed = ledger_launch(path, sleeper(0.1), env)
    out["suspension"] = {"launch_while_suspended": refused, "resumed": resumed["admitted"],
                         "charged_during_suspension": round(ledger_load(path)["used"]["active_seconds"] - before, 2)}

    # Crash mid-worker: a separate coordinator is SIGKILLed; recovery kills the
    # orphaned group, keeps the invocation charge, and adds a bounded tail.
    crash = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "--ledger-launch", str(path)])
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        flight = ledger_load(path).get("in_flight")
        if flight and flight.get("pgid"):
            break
        time.sleep(0.05)
    time.sleep(1.2)
    crash.send_signal(signal.SIGKILL)
    crash.wait()
    orphan_pgid = ledger_load(path)["in_flight"]["pgid"]
    pre = ledger_load(path)["used"]
    orphan_alive_before_recovery = alive(orphan_pgid)
    recovered = ledger_recover(path)
    out["crash"] = {"used_before_recovery": pre, "used_after_recovery": recovered["used"],
                    "orphan_alive_before_recovery": orphan_alive_before_recovery,
                    "orphan_alive_after_recovery": alive(orphan_pgid),
                    "state": recovered["state"], "event": recovered["events"][-1]}
    # Restart does not reset the allowance; admission requires operator resumption.
    out["restart_refused_until_resumed"] = ledger_launch(path, sleeper(0), env)
    ledger_transition(path, "active", "operator resumed after crash")

    # Exhaustion, then a recorded operator grant.
    while ledger_launch(path, sleeper(0), env)["admitted"]:
        pass
    exhausted = ledger_load(path)
    out["exhaustion"] = {"state": exhausted["state"], "used": exhausted["used"],
                         "limits": exhausted["limits"]}
    out["relaunch_after_exhaustion"] = ledger_launch(path, sleeper(0), env)
    ledger_grant(path, operator="fixture-operator", invocations=1)
    ledger_transition(path, "active", "operator granted one invocation")
    out["after_grant"] = ledger_launch(path, sleeper(0), env)
    out["final"] = {k: v for k, v in ledger_load(path).items() if k != "events"}
    out["event_kinds"] = [e["kind"] for e in ledger_load(path)["events"]]
    return out


# --------------------------------------------------------------------------

def run():
    if os.name == "nt":
        raise RuntimeError("Unix fixture; see probe_tree_windows.py for native Windows")
    binary = os.environ.get("CLAUDINE_BIN") or shutil.which("claudine")
    if not binary:
        raise RuntimeError("set CLAUDINE_BIN to the Claudine build under test")
    out_path = Path(sys.argv[sys.argv.index("--out") + 1]) if "--out" in sys.argv else ROOT / "probe-results.json"
    with tempfile.TemporaryDirectory(prefix="messenger-budget-probe-") as directory:
        workspace = Path(directory).resolve()
        home = workspace / "home"
        home.mkdir()
        bin_dir = workspace / "bin"
        bin_dir.mkdir()
        fake = bin_dir / "goose"
        fake.write_text("#!" + sys.executable + "\n" + Path(__file__).read_text())
        fake.chmod(0o700)
        env = {"HOME": str(home), "USERPROFILE": str(home),
               "PATH": os.pathsep.join([str(bin_dir), "/usr/bin", "/bin"]),
               "LANG": "C.UTF-8", "NO_COLOR": "1", "TERM": "dumb",
               "CLAUDINE_RENDEZVOUS_REPORT": "false", "PLAYA_DRY_RUN": "1",
               "PLAYA_SPOOL_DIR": str(workspace / "audio"),
               "GIT_CONFIG_GLOBAL": "/dev/null",
               "PROBE_RECORDS": str(workspace / "records.jsonl"),
               # Sensitive-looking names: is either admitted to the worker?
               "FIXTURE_SECRET_TOKEN": "not-a-secret", "OPENAI_API_KEY": "not-a-key"}
        result = {
            "binary": binary,
            "binary_version": subprocess.run([binary, "--version"], capture_output=True, text=True).stdout.strip(),
            "source_revision": os.environ.get("SOURCE_REVISION", "unverified"),
            "host": " ".join(os.uname()),
            "native": native_probes(binary, workspace, env),
            "prototype_ledger_only": ledger_probes(workspace),
        }
        for probe in result["native"].values():
            for row in probe.get("worker", []) if isinstance(probe, dict) else []:
                row["cwd"] = row["cwd"].replace(str(workspace), "<tmp>")
                row["instruction_files_visible"] = [p.replace(str(workspace), "<tmp>") for p in row["instruction_files_visible"]]
    out_path.write_text(json.dumps(result, indent=2) + "\n")
    print(out_path)


if __name__ == "__main__":
    if Path(sys.argv[0]).name == "goose":
        fake_provider()
    elif "--ledger-launch" in sys.argv:
        ledger_launch(Path(sys.argv[sys.argv.index("--ledger-launch") + 1]),
                      [sys.executable, "-c", "import time; time.sleep(30)"], {"PATH": os.environ.get("PATH", "")})
    else:
        run()
