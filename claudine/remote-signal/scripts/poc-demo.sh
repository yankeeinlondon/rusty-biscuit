#!/usr/bin/env bash
# Phase-6 POC demo script.
#
# Spins up two daemons in the background on temporary data directories,
# pairs them via a Bech32 invitation, drives a few session-log appends on
# each side, runs the direct-sync protocol, and asserts that both
# daemons converge on the same set of entries. Tear-down kills both
# daemons and removes the temporary state.
#
# This script is intentionally dependency-free beyond the
# `remote-signal-daemon` and `remote-signal-test-client` binaries — it
# uses bash + the test client only. Run it after `just build` from the
# `claudine/remote-signal/` package area, or pass the binaries via
# `RS_DAEMON_BIN` / `RS_CLIENT_BIN`.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/../../target}"
DAEMON_BIN="${RS_DAEMON_BIN:-$TARGET_DIR/debug/remote-signal-daemon}"
CLIENT_BIN="${RS_CLIENT_BIN:-$TARGET_DIR/debug/remote-signal-test-client}"

if [[ ! -x "$DAEMON_BIN" ]]; then
    echo "remote-signal-daemon not found at $DAEMON_BIN" >&2
    echo "Run 'just build' from the remote-signal/ package area first, or set RS_DAEMON_BIN." >&2
    exit 1
fi
if [[ ! -x "$CLIENT_BIN" ]]; then
    echo "remote-signal-test-client not found at $CLIENT_BIN" >&2
    echo "Run 'just build' from the remote-signal/ package area first, or set RS_CLIENT_BIN." >&2
    exit 1
fi

WORK_DIR="$(mktemp -d -t rs-poc.XXXXXX)"
ALICE_SOCK="$WORK_DIR/alice.sock"
BOB_SOCK="$WORK_DIR/bob.sock"
ALICE_DATA="$WORK_DIR/alice-data"
BOB_DATA="$WORK_DIR/bob-data"
ALICE_LOG="$WORK_DIR/alice.log"
BOB_LOG="$WORK_DIR/bob.log"

cleanup() {
    local status=$?
    if [[ -n "${ALICE_PID:-}" ]] && kill -0 "$ALICE_PID" 2>/dev/null; then
        kill "$ALICE_PID" 2>/dev/null || true
        wait "$ALICE_PID" 2>/dev/null || true
    fi
    if [[ -n "${BOB_PID:-}" ]] && kill -0 "$BOB_PID" 2>/dev/null; then
        kill "$BOB_PID" 2>/dev/null || true
        wait "$BOB_PID" 2>/dev/null || true
    fi
    if [[ $status -ne 0 ]]; then
        echo "----- alice.log -----"
        cat "$ALICE_LOG" 2>/dev/null || true
        echo "----- bob.log -----"
        cat "$BOB_LOG" 2>/dev/null || true
    fi
    rm -rf "$WORK_DIR"
    exit $status
}
trap cleanup EXIT INT TERM

say() { printf '\n\033[1;36m▶ %s\033[0m\n' "$*"; }

wait_for_socket() {
    local path="$1"
    local deadline=$(( $(date +%s) + 5 ))
    while [[ ! -S "$path" ]]; do
        if (( $(date +%s) >= deadline )); then
            echo "socket $path never appeared" >&2
            return 1
        fi
        sleep 0.1
    done
}

alice() {
    "$CLIENT_BIN" --socket "$ALICE_SOCK" "$@"
}

bob() {
    "$CLIENT_BIN" --socket "$BOB_SOCK" "$@"
}

say "1. Starting daemons (Alice + Bob) on $WORK_DIR"
"$DAEMON_BIN" \
    --socket "$ALICE_SOCK" \
    --data-dir "$ALICE_DATA" \
    --in-memory-projection \
    --quic-bind 127.0.0.1:0 \
    --no-mdns \
    >"$ALICE_LOG" 2>&1 &
ALICE_PID=$!
"$DAEMON_BIN" \
    --socket "$BOB_SOCK" \
    --data-dir "$BOB_DATA" \
    --in-memory-projection \
    --quic-bind 127.0.0.1:0 \
    --no-mdns \
    >"$BOB_LOG" 2>&1 &
BOB_PID=$!

wait_for_socket "$ALICE_SOCK"
wait_for_socket "$BOB_SOCK"

say "2. Ping check"
alice ping --nonce alice-hello
bob ping --nonce bob-hello

say "3. Resolve node identities + grab Bob's invitation"
ALICE_INVITATION_LINE=$(alice create-invitation)
BOB_INVITATION_LINE=$(bob create-invitation)
ALICE_NODE=$(echo "$ALICE_INVITATION_LINE" | sed -n 's/.* node_id=\([0-9a-f]\{64\}\) .*/\1/p')
BOB_NODE=$(echo "$BOB_INVITATION_LINE" | sed -n 's/.* node_id=\([0-9a-f]\{64\}\) .*/\1/p')
BOB_INVITATION=$(echo "$BOB_INVITATION_LINE" | sed -n 's/^invitation=\([^ ]*\) .*/\1/p')
echo "    alice = $ALICE_NODE"
echo "    bob   = $BOB_NODE"

say "4. Pair daemons (mutual approval)"
alice approve-peer --node-id "$BOB_NODE" --note "bob"
bob approve-peer --node-id "$ALICE_NODE" --note "alice"

say "5. Alice dials Bob's invitation"
alice connect-to-peer --invitation "$BOB_INVITATION"

say "6. Append entries on both sides"
for i in 0 1 2; do
    alice append-entry --owner-node-id shared --session-id demo --source alice --message "alice-$i"
done
for i in 0 1; do
    bob append-entry --owner-node-id shared --session-id demo --source bob --message "bob-$i"
done

say "7. Run direct sync from Alice"
alice sync-with-peer --node-id "$BOB_NODE"

# Give the responder side a moment to finish applying deltas. The
# sync RPC awaits the bidi exchange itself, but list_chunk_entries
# reads the materialised Loro state which is updated synchronously.
sleep 0.1

say "8. Verify both sides observe the same entries"
collect_messages() {
    local sock="$1"
    local owner="$2"
    local session="$3"
    "$CLIENT_BIN" --socket "$sock" list-chunks --owner-node-id "$owner" --session-id "$session" |
        while read -r chunk; do
            [[ -z "$chunk" ]] && continue
            "$CLIENT_BIN" --socket "$sock" list-entries --chunk-id "$chunk" |
                sed -n 's/.* message=\(.*\)$/\1/p'
        done | sort -u
}

ALICE_MSGS=$(collect_messages "$ALICE_SOCK" shared demo)
BOB_MSGS=$(collect_messages "$BOB_SOCK" shared demo)

echo "    Alice sees:"
echo "$ALICE_MSGS" | sed 's/^/      - /'
echo "    Bob sees:"
echo "$BOB_MSGS" | sed 's/^/      - /'

EXPECTED=$(printf '%s\n' alice-0 alice-1 alice-2 bob-0 bob-1 | sort -u)
if [[ "$ALICE_MSGS" != "$EXPECTED" ]]; then
    echo "Alice's messages do not match expected set" >&2
    exit 1
fi
if [[ "$BOB_MSGS" != "$EXPECTED" ]]; then
    echo "Bob's messages do not match expected set" >&2
    exit 1
fi

say "POC demo succeeded — both daemons converged on the same set of entries."
