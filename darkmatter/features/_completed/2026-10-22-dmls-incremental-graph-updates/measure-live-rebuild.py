#!/usr/bin/env python3
"""Scripted DMLS editing sample for live-rebuild cost measurement.

Drives a release `dmls` binary over framed stdio LSP against a repository
checkout, then summarizes the `DMLS_MEASURE` lines a temporary timing seam in
`WorkspaceIndex::rebuild` prints to stderr. The seam is not part of the
shipped code; without it the driver still runs but the summary is empty.

Usage:

    measure-live-rebuild.py run --dmls target/release/dmls --root . \
        --out /tmp/dmls-measure [--source darkmatter/dmls/src/graph/invalidate.rs]

`run` writes `events.jsonl` (driver events: times, methods, versions,
statuses; never document text), `stderr.log` (server stderr with driver
timestamps and phase), and `summary.json`. `--source` maps the seam's
`L<line>` causes to the `WorkspaceIndex` method that requested the rebuild;
pass the instrumented file the binary was built from.

The session: initialize with dynamic watched-file registration and
work-done progress; wait for startup to settle; then, timed, open three
documents, type about 250 single characters across them at six per second
with pauses, send a hover and a completion, send two save notifications with
matching watched-file change events for the real (unchanged) disk files, add
and remove one listed Markdown file, create 50 unlisted files under `target/`
in one notification, and close one buffer. Driver-created files are uniquely
named and removed on teardown. The whole run has a 180 s deadline.
"""

from __future__ import annotations

import argparse
import json
import os
import queue
import re
import shutil
import signal
import statistics
import subprocess
import sys
import threading
import time
import uuid
from pathlib import Path

DEADLINE_S = 180.0
REQUEST_TIMEOUT_S = 10.0
TYPE_RATE_HZ = 6.0
DOCUMENTS = ["AGENTS.md", "darkmatter/README.md", "darkmatter/docs/lsp/indexing.md"]
STARTUP_TOKEN = "dmls/startup-index"
SETTLE_QUIET_S = 3.0


class Session:
    def __init__(self, dmls: Path, root: Path, out: Path):
        self.root = root
        self.out = out
        self.started = time.monotonic()
        self.phase = "startup"
        self.events = open(out / "events.jsonl", "w")
        self.stderr_log = open(out / "stderr.log", "w")
        self.proc = subprocess.Popen(
            [str(dmls), "--stdio"],
            cwd=root,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.next_id = 1
        self.responses: dict[int, queue.Queue] = {}
        self.lock = threading.Lock()
        self.last_message = time.monotonic()
        self.progress_open: set[str] = set()
        self.startup_done = threading.Event()
        self.diagnostics = 0
        threading.Thread(target=self._read_stdout, daemon=True).start()
        threading.Thread(target=self._read_stderr, daemon=True).start()

    def now(self) -> float:
        return time.monotonic() - self.started

    def log(self, **event) -> None:
        event = {"t": round(self.now(), 3), "phase": self.phase, **event}
        self.events.write(json.dumps(event) + "\n")
        self.events.flush()

    # Framing -----------------------------------------------------------

    def send(self, message: dict) -> None:
        message = {"jsonrpc": "2.0", **message}
        body = json.dumps(message).encode()
        with self.lock:
            self.proc.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
            self.proc.stdin.flush()

    def notify(self, method: str, params: dict, **detail) -> None:
        self.send({"method": method, "params": params})
        self.log(kind="notify", method=method, **detail)

    def request(self, method: str, params: dict) -> dict | None:
        request_id = self.next_id
        self.next_id += 1
        waiter: queue.Queue = queue.Queue()
        self.responses[request_id] = waiter
        sent = time.monotonic()
        self.send({"id": request_id, "method": method, "params": params})
        try:
            response = waiter.get(timeout=REQUEST_TIMEOUT_S)
        except queue.Empty:
            self.log(kind="request", method=method, status="timeout")
            return None
        status = "error" if "error" in response else "ok"
        self.log(kind="request", method=method, status=status, ms=round((time.monotonic() - sent) * 1000, 1))
        return response

    def _read_stdout(self) -> None:
        stream = self.proc.stdout
        while True:
            length = None
            while True:
                line = stream.readline()
                if not line:
                    return
                line = line.strip()
                if not line:
                    break
                if line.lower().startswith(b"content-length:"):
                    length = int(line.split(b":", 1)[1])
            if length is None:
                continue
            message = json.loads(stream.read(length))
            self.last_message = time.monotonic()
            self._dispatch(message)

    def _dispatch(self, message: dict) -> None:
        method = message.get("method")
        if method is None:
            waiter = self.responses.pop(message.get("id"), None)
            if waiter is not None:
                waiter.put(message)
            return
        if "id" in message:
            result = None
            if method == "workspace/configuration":
                result = [None for _ in message.get("params", {}).get("items", [])]
            self.send({"id": message["id"], "result": result})
            self.log(kind="server-request", method=method, status="acknowledged")
            return
        if method == "$/progress":
            params = message.get("params", {})
            token = str(params.get("token"))
            progress = params.get("value", {}).get("kind")
            if progress == "begin":
                self.progress_open.add(token)
            elif progress == "end":
                self.progress_open.discard(token)
                if token == STARTUP_TOKEN:
                    self.startup_done.set()
            if progress in ("begin", "end"):
                self.log(kind="progress", token=token, status=progress)
        elif method == "textDocument/publishDiagnostics":
            self.diagnostics += 1

    def _read_stderr(self) -> None:
        for raw in self.proc.stderr:
            line = raw.decode(errors="replace").rstrip("\n")
            self.stderr_log.write(f"{self.now():.3f}\t{self.phase}\t{line}\n")
            self.stderr_log.flush()

    # Session helpers ---------------------------------------------------

    def uri(self, relative: str) -> str:
        return (self.root / relative).as_uri()

    def watched(self, changes: list[tuple[Path, int]], label: str) -> None:
        params = {"changes": [{"uri": path.as_uri(), "type": kind} for path, kind in changes]}
        self.notify("workspace/didChangeWatchedFiles", params, label=label, count=len(changes))

    def barrier(self, relative: str) -> None:
        self.request("textDocument/documentSymbol", {"textDocument": {"uri": self.uri(relative)}})

    def wait_settled(self, deadline: float) -> bool:
        while time.monotonic() < deadline:
            if (
                self.startup_done.is_set()
                and not self.progress_open
                and time.monotonic() - self.last_message >= SETTLE_QUIET_S
            ):
                return True
            time.sleep(0.2)
        return False

    def process_cpu_s(self) -> float | None:
        try:
            text = subprocess.run(
                ["ps", "-o", "time=", "-p", str(self.proc.pid)], capture_output=True, text=True, timeout=5
            ).stdout.strip()
        except (OSError, subprocess.TimeoutExpired):
            return None
        match = re.fullmatch(r"(?:(\d+)-)?(?:(\d+):)?(\d+):(\d+(?:\.\d+)?)", text)
        if not match:
            return None
        days, hours, minutes, seconds = match.groups()
        return int(days or 0) * 86400 + int(hours or 0) * 3600 + int(minutes) * 60 + float(seconds)


class Buffer:
    """An open document whose edits append characters to its last line."""

    def __init__(self, session: Session, relative: str):
        self.session = session
        self.relative = relative
        self.text = (session.root / relative).read_text()
        self.version = 1
        session.notify(
            "textDocument/didOpen",
            {"textDocument": {"uri": session.uri(relative), "languageId": "markdown", "version": 1, "text": self.text}},
            version=1,
        )

    def end_position(self) -> dict:
        lines = self.text.split("\n")
        return {"line": len(lines) - 1, "character": len(lines[-1].encode("utf-16-le")) // 2}

    def type(self, count: int) -> None:
        if not self.text.endswith("\n\n"):
            self._change(self.text + ("\n" if self.text.endswith("\n") else "\n\n"))
        for index in range(count):
            self._change(self.text + "abcdefghij"[index % 10])
            time.sleep(1.0 / TYPE_RATE_HZ)

    def _change(self, text: str) -> None:
        self.text = text
        self.version += 1
        self.session.notify(
            "textDocument/didChange",
            {"textDocument": {"uri": self.session.uri(self.relative), "version": self.version}, "contentChanges": [{"text": text}]},
            version=self.version,
        )

    def save(self) -> None:
        # The disk file is never overwritten: the save and its watched event
        # report the real, unchanged file.
        self.session.notify("textDocument/didSave", {"textDocument": {"uri": self.session.uri(self.relative)}})
        self.session.watched([(self.session.root / self.relative, 2)], label="save")

    def close(self) -> None:
        self.session.notify("textDocument/didClose", {"textDocument": {"uri": self.session.uri(self.relative)}})


def run(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    tag = uuid.uuid4().hex[:12]
    joined = root / "darkmatter" / "docs" / "lsp" / f"zz-measure-{tag}.md"
    build_dir = root / "target" / f"measure-{tag}"
    session = Session(Path(args.dmls).resolve(), root, out)
    hard_deadline = time.monotonic() + DEADLINE_S

    def watchdog() -> None:
        while session.proc.poll() is None:
            if time.monotonic() > hard_deadline:
                session.log(kind="deadline", status="killed")
                session.proc.kill()
                return
            time.sleep(0.5)

    threading.Thread(target=watchdog, daemon=True).start()
    summary: dict = {"tag": tag}
    status = 1
    try:
        initialize = session.request(
            "initialize",
            {
                "processId": os.getpid(),
                "rootUri": root.as_uri(),
                "workspaceFolders": [{"uri": root.as_uri(), "name": root.name}],
                "capabilities": {
                    "workspace": {"didChangeWatchedFiles": {"dynamicRegistration": True}, "configuration": True},
                    "window": {"workDoneProgress": True},
                    "textDocument": {"hover": {}, "completion": {}, "publishDiagnostics": {}, "synchronization": {"didSave": True}},
                },
            },
        )
        if initialize is None:
            raise RuntimeError("initialize timed out")
        session.notify("initialized", {})
        if not session.wait_settled(hard_deadline - 115):
            raise RuntimeError("startup did not settle")

        session.phase = "sample"
        cpu_before = session.process_cpu_s()
        sample_started = time.monotonic()
        summary["sample_start_s"] = round(session.now(), 3)
        first, second, third = (Buffer(session, path) for path in DOCUMENTS)
        time.sleep(2)
        first.type(45)
        time.sleep(4)
        session.request("textDocument/hover", {"textDocument": {"uri": session.uri(first.relative)}, "position": {"line": 0, "character": 2}})
        second.type(45)
        time.sleep(4)
        session.request("textDocument/completion", {"textDocument": {"uri": session.uri(second.relative)}, "position": second.end_position()})
        first.save()
        third.type(40)
        time.sleep(3)
        joined.write_text("# Measurement join\n\nA driver-owned listed file.\n")
        session.watched([(joined, 1)], label="listed-join")
        session.barrier(first.relative)
        first.type(40)
        time.sleep(3)
        build_dir.mkdir(parents=True)
        created = []
        for index in range(50):
            path = build_dir / f"output-{index:02}.md"
            path.write_text("generated\n")
            created.append((path, 1))
        session.watched(created, label="build-output")
        session.barrier(first.relative)
        second.type(40)
        second.save()
        time.sleep(3)
        joined.unlink()
        session.watched([(joined, 3)], label="listed-leave")
        session.barrier(first.relative)
        third.type(40)
        third.close()
        remaining = 100.0 - (time.monotonic() - sample_started)
        if remaining > 0:
            time.sleep(remaining)
        session.barrier(first.relative)
        summary["sample_duration_s"] = round(time.monotonic() - sample_started, 3)
        summary["sample_end_s"] = round(session.now(), 3)
        cpu_after = session.process_cpu_s()
        summary["process_cpu_s"] = None if cpu_before is None or cpu_after is None else round(cpu_after - cpu_before, 3)
        summary["diagnostics_notifications"] = session.diagnostics

        session.phase = "teardown"
        session.request("shutdown", {})
        session.notify("exit", {})
        try:
            session.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            session.proc.terminate()
        status = 0
    except Exception as error:  # noqa: BLE001 - report and clean up
        summary["error"] = f"{type(error).__name__}: {error}"
        session.log(kind="failure", status=summary["error"])
    finally:
        if session.proc.poll() is None:
            session.proc.terminate()
            try:
                session.proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                session.proc.kill()
        if joined.exists():
            joined.unlink()
        shutil.rmtree(build_dir, ignore_errors=True)
        time.sleep(0.5)
        session.stderr_log.flush()
        session.events.close()
    summary.update(analyze(out / "stderr.log", Path(args.source) if args.source else None, summary))
    (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    return status


def cause_names(source: Path | None) -> dict[int, str]:
    """Maps each line of `source` to the enclosing `fn`'s name."""
    if source is None:
        return {}
    names: dict[int, str] = {}
    current = "?"
    for number, line in enumerate(source.read_text().splitlines(), start=1):
        match = re.match(r"\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)", line)
        if match:
            current = match.group(1)
        names[number] = current
    return names


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(round(fraction * (len(ordered) - 1))))]


def analyze(stderr_log: Path, source: Path | None, summary: dict) -> dict:
    names = cause_names(source)
    rebuilds = []
    for line in stderr_log.read_text().splitlines():
        t, phase, text = line.split("\t", 2)
        if "DMLS_MEASURE" not in text:
            continue
        fields = dict(part.split("=", 1) for part in text.split("DMLS_MEASURE", 1)[1].split())
        causes = [names.get(int(cause[1:]), cause) for cause in fields["causes"].split(",")]
        stages = {}
        for stage in filter(None, fields["stages"].split(",")):
            name, wall, cpu = stage.split(":")
            stages[name] = (int(wall) / 1e6, None if int(cpu) >= 2**127 else int(cpu) / 1e6)
        rebuilds.append(
            {
                "t": float(t),
                "phase": phase,
                "causes": causes,
                "docs": int(fields["docs"]),
                "nodes": int(fields["nodes"]),
                "edges": int(fields["edges"]),
                "wall_ms": int(fields["wall_ns"]) / 1e6,
                "cpu_ms": None if fields["cpu_ns"] == "ERR" else int(fields["cpu_ns"]) / 1e6,
                "stages": stages,
            }
        )
    sample = [rebuild for rebuild in rebuilds if rebuild["phase"] == "sample"]
    result: dict = {
        "rebuilds_startup": sum(1 for rebuild in rebuilds if rebuild["phase"] == "startup"),
        "rebuilds_sample": len(sample),
        "rebuilds_teardown": sum(1 for rebuild in rebuilds if rebuild["phase"] == "teardown"),
    }
    if not sample:
        return result
    cpu_failed = any(rebuild["cpu_ms"] is None for rebuild in sample)
    result["cpu_clock_failed"] = cpu_failed
    by_cause: dict[str, list[dict]] = {}
    for rebuild in sample:
        key = "+".join(sorted(set(rebuild["causes"]))) if len(rebuild["causes"]) > 1 else rebuild["causes"][0]
        by_cause.setdefault(key, []).append(rebuild)
    result["by_cause"] = {
        cause: {
            "rebuilds": len(group),
            "cpu_median_ms": None if cpu_failed else round(statistics.median(r["cpu_ms"] for r in group), 3),
            "cpu_total_s": None if cpu_failed else round(sum(r["cpu_ms"] for r in group) / 1000, 4),
            "wall_median_ms": round(statistics.median(r["wall_ms"] for r in group), 3),
        }
        for cause, group in sorted(by_cause.items(), key=lambda item: -len(item[1]))
    }
    walls = [rebuild["wall_ms"] for rebuild in sample]
    result["wall_median_ms"] = round(statistics.median(walls), 3)
    result["wall_p90_ms"] = round(percentile(walls, 0.9), 3)
    result["wall_max_ms"] = round(max(walls), 3)
    result["wall_over_50ms"] = sum(1 for wall in walls if wall > 50)
    if not cpu_failed:
        cpus = [rebuild["cpu_ms"] for rebuild in sample]
        result["cpu_median_ms"] = round(statistics.median(cpus), 3)
        result["cpu_p90_ms"] = round(percentile(cpus, 0.9), 3)
        result["cpu_total_s"] = round(sum(cpus) / 1000, 4)
        duration = summary.get("sample_duration_s")
        if duration:
            result["rebuild_cpu_share_of_one_core_pct"] = round(100 * sum(cpus) / 1000 / duration, 3)
    else:
        duration = summary.get("sample_duration_s")
        if duration:
            result["rebuild_wall_occupied_share_pct_estimate"] = round(100 * sum(walls) / 1000 / duration, 3)
    stage_names = sorted({name for rebuild in sample for name in rebuild["stages"]})
    result["stages"] = {}
    for name in stage_names:
        values = [rebuild["stages"][name] for rebuild in sample if name in rebuild["stages"]]
        cpus = [cpu for _, cpu in values if cpu is not None]
        result["stages"][name] = {
            "count": len(values),
            "wall_median_ms": round(statistics.median(wall for wall, _ in values), 3),
            "cpu_median_ms": round(statistics.median(cpus), 3) if cpus else None,
            "cpu_total_ms": round(sum(cpus), 3) if cpus else None,
        }
    for key in ("docs", "nodes", "edges"):
        result[f"{key}_range"] = [min(r[key] for r in sample), max(r[key] for r in sample)]
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    run_parser = commands.add_parser("run")
    run_parser.add_argument("--dmls", required=True)
    run_parser.add_argument("--root", required=True)
    run_parser.add_argument("--out", required=True)
    run_parser.add_argument("--source")
    analyze_parser = commands.add_parser("analyze")
    analyze_parser.add_argument("--out", required=True)
    analyze_parser.add_argument("--source")
    args = parser.parse_args()
    if args.command == "run":
        signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
        return run(args)
    out = Path(args.out)
    summary = json.loads((out / "summary.json").read_text())
    summary.update(analyze(out / "stderr.log", Path(args.source) if args.source else None, summary))
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
