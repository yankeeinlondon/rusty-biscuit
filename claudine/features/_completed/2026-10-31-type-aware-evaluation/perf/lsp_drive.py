#!/usr/bin/env python3
# Phase 1 and Phase 8 performance driver: times DMLS didOpen -> publishDiagnostics over stdio.
# Usage: python3 -I lsp_drive.py <dmls> <document> <workspace-root> <runs>
import json, subprocess, sys, time, os, pathlib
dmls, doc, root, runs = sys.argv[1], pathlib.Path(sys.argv[2]).resolve(), pathlib.Path(sys.argv[3]).resolve(), int(sys.argv[4])
def send(p, msg):
    b = json.dumps(msg).encode()
    p.stdin.write(b"Content-Length: %d\r\n\r\n" % len(b) + b); p.stdin.flush()
def recv(p):
    h = b""
    while not h.endswith(b"\r\n\r\n"):
        c = p.stdout.read(1)
        if not c: return None
        h += c
    n = int([l for l in h.split(b"\r\n") if l.lower().startswith(b"content-length")][0].split(b":")[1])
    return json.loads(p.stdout.read(n))
uri = doc.as_uri(); text = doc.read_text()
results = []
for i in range(runs):
    log = f"/tmp/p1-perf/dmls-{i}.log"
    if os.path.exists(log): os.remove(log)
    p = subprocess.Popen([dmls, "--log-level", "debug", "--log-file", log], stdin=subprocess.PIPE, stdout=subprocess.PIPE, cwd=str(root))
    send(p, {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":os.getpid(),"rootUri":root.as_uri(),"capabilities":{"textDocument":{"publishDiagnostics":{}}},"workspaceFolders":[{"uri":root.as_uri(),"name":"r"}]}})
    while True:
        m = recv(p)
        if m.get("id") == 1: break
    send(p, {"jsonrpc":"2.0","method":"initialized","params":{}})
    t0 = time.perf_counter()
    send(p, {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"languageId":"markdown","version":1,"text":text}}})
    diags = None; first=None; seen=0
    while True:
        m = recv(p)
        if m is None: break
        if m.get("method") == "window/workDoneProgress/create" or ("id" in m and "method" in m):
            send(p, {"jsonrpc":"2.0","id":m["id"],"result":None}); continue
        if m.get("method") == "textDocument/publishDiagnostics" and m["params"]["uri"] == uri:
            diags = m["params"]["diagnostics"]; seen+=1
            if seen==1: first=(time.perf_counter()-t0)*1000
            if seen==2: break
    dt = (time.perf_counter() - t0) * 1000
    send(p, {"jsonrpc":"2.0","id":2,"method":"shutdown"})
    while True:
        m = recv(p)
        if m is None or m.get("id") == 2: break
    send(p, {"jsonrpc":"2.0","method":"exit"}); p.wait(timeout=10)
    results.append((first, len(diags or []), dt-first))
for dt, n, w in results: print(f"open->first publish {dt:.1f} ms, second (cache-hit) publish +{w:.1f} ms, {n} diagnostics")
ds = sorted(r[0] for r in results); print(f"median {ds[len(ds)//2]:.1f} ms min {ds[0]:.1f} max {ds[-1]:.1f}")
