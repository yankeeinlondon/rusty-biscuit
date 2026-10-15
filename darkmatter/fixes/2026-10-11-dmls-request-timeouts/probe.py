"""Bounded headless baseline probe; outputs metrics, never document bodies or logs."""
import argparse
import json
import pathlib
import queue
import subprocess
import threading
import time


def scenario(binary, root, document, rounds, wait_startup=False):
    started = time.perf_counter()
    deadline = started + 150
    inbox = queue.Queue()
    notes = []
    startup_done = threading.Event()
    child = subprocess.Popen([str(binary), "--stdio", "--log-level", "info"],
                             cwd=root, stdin=subprocess.PIPE,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    watchdog = threading.Timer(150, lambda: child.kill() if child.poll() is None else None)
    watchdog.start()
    lock = threading.Lock()
    def send(message):
        payload = json.dumps({"jsonrpc": "2.0", **message}).encode()
        with lock:
            child.stdin.write(f"Content-Length: {len(payload)}\r\n\r\n".encode() + payload)
            child.stdin.flush()
    def reader():
        while True:
            line = child.stdout.readline()
            if not line:
                return
            length = int(line.split(b":", 1)[1])
            while child.stdout.readline().strip():
                pass
            message = json.loads(child.stdout.read(length))
            if "method" in message and "id" in message:
                send({"id": message["id"], "result": None})
            else:
                inbox.put((time.perf_counter(), message))
    def stderr_reader():
        for line in iter(child.stderr.readline, b""):
            if b"startup index complete" in line:
                import re
                count = re.search(rb"documents=(\d+)", line)
                notes.append({"startup_ms": (time.perf_counter()-started)*1000,
                              "indexed_documents": int(count[1]) if count else None})
                startup_done.set()
    threading.Thread(target=reader, daemon=True).start()
    threading.Thread(target=stderr_reader, daemon=True).start()
    serial = 0
    def request(method, params, cancel=False):
        nonlocal serial
        serial += 1
        sent = time.perf_counter()
        send({"id": serial, "method": method, "params": params})
        if cancel:
            send({"method": "$/cancelRequest", "params": {"id": serial}})
        while True:
            arrived, message = inbox.get(timeout=max(0.001, deadline-time.perf_counter()))
            if message.get("id") == serial:
                return {"method": method, "elapsed_ms": round((arrived-sent)*1000, 3),
                        "error_code": message.get("error", {}).get("code"),
                        "cancel_sent": cancel}
    results = []
    try:
        results.append(request("initialize", {"processId": None, "rootUri": root.as_uri(),
                       "capabilities": {"workspace": {"didChangeWatchedFiles":
                                                       {"dynamicRegistration": True}}}}))
        send({"method": "initialized", "params": {}})
        if wait_startup and not startup_done.wait(timeout=max(0.001, deadline-time.perf_counter())):
            raise queue.Empty
        unrelated_uri = "untitled:responsiveness-probe.md"
        send({"method": "textDocument/didOpen", "params": {"textDocument":
             {"uri": unrelated_uri, "languageId": "markdown", "version": 1, "text": "# Probe\n"}}})
        warmup = request("textDocument/foldingRange", {"textDocument": {"uri": unrelated_uri}})
        warmup.update(phase="warmup", unrelated=True)
        results.append(warmup)
        text = document.read_text()
        uri = document.as_uri()
        sent_open = time.perf_counter()
        send({"method": "textDocument/didOpen", "params": {"textDocument":
             {"uri": uri, "languageId": "markdown", "version": 1, "text": text}}})
        td = {"textDocument": {"uri": uri}}
        positioned = {**td, "position": {"line": 0, "character": 0}}
        methods = [("textDocument/documentHighlight", positioned),
                   ("textDocument/codeAction", {**td, "range": {"start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 0}}, "context": {"diagnostics": []}}),
                   ("textDocument/semanticTokens/full", td),
                   ("textDocument/foldingRange", td), ("textDocument/documentLink", td),
                   ("textDocument/hover", positioned), ("textDocument/completion", positioned)]
        for phase in ("open", "edit", "burst20", "cancel"):
            phase_started = time.perf_counter()
            if phase in ("edit", "burst20"):
                for version in range(2, 3 if phase == "edit" else 22):
                    send({"method": "textDocument/didChange", "params": {"textDocument":
                         {"uri": uri, "version": version + (1 if phase == "burst20" else 0)},
                         "contentChanges": [{"text": text + "\n" * version}]}})
            sends_complete = time.perf_counter()
            for iteration in range(rounds):
                metric = request("textDocument/foldingRange", {"textDocument": {"uri": unrelated_uri}})
                if iteration == 0:
                    notes.append({"phase": phase, "send_ms": round((sends_complete-phase_started)*1000, 3),
                                  "trigger_to_first_unrelated_response_ms": round((time.perf_counter()-phase_started)*1000, 3)})
                metric.update(phase=phase, iteration=iteration, unrelated=True)
                results.append(metric)
                for method, params in methods:
                    metric = request(method, params, phase == "cancel")
                    metric.update(phase=phase, iteration=iteration)
                    results.append(metric)
                    if phase == "open" and iteration == 0 and method == methods[0][0]:
                        notes.append({"open_to_first_response_ms": round((time.perf_counter()-sent_open)*1000, 3)})
        request("shutdown", None)
        send({"method": "exit", "params": None})
        child.stdin.close()
        child.wait(timeout=max(0.001, deadline-time.perf_counter()))
        resources = {}
        try:
            import resource
            import sys
            usage = resource.getrusage(resource.RUSAGE_CHILDREN)
            resources = {"peak_rss_bytes": usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024),
                         "cpu_user_seconds": usage.ru_utime, "cpu_system_seconds": usage.ru_stime}
        except ImportError:
            pass
        return {"document_bytes": len(text.encode()), "duration_ms": round((time.perf_counter()-started)*1000, 3),
                "measurements": results, "notes": notes, "exit_code": child.returncode, "resources": resources}
    except (queue.Empty, subprocess.TimeoutExpired, BrokenPipeError):
        return {"bounded_timeout": True, "measurements": results, "notes": notes}
    finally:
        watchdog.cancel()
        if child.poll() is None:
            child.kill()
            child.wait()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", type=pathlib.Path)
    parser.add_argument("root", type=pathlib.Path)
    parser.add_argument("document", type=pathlib.Path)
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--wait-startup", action="store_true")
    args = parser.parse_args()
    print(json.dumps(scenario(args.binary.resolve(), args.root.resolve(), args.document.resolve(), args.rounds, args.wait_startup), indent=2))
