#!/bin/sh
# Reads one page in a headless browser and writes the probe's JSON.
# usage: measure.sh mac|linux <work dir> <html path relative to work dir> <out json>
# `linux` runs Debian Chromium in Docker (image `rb-ac-e1-chromium`, built from
# the Dockerfile here); the work dir must be under $HOME, since Docker Desktop
# cannot mount /tmp.
reader=$1; work=$2; file=$3; out=$4
for attempt in 1 2 3; do
  if [ "$reader" = mac ]; then
    profile=$(mktemp -d)
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --disable-gpu \
      --user-data-dir="$profile" --virtual-time-budget=15000 --dump-dom "file://$work/$file" 2>/dev/null > "$out.dom"
    rm -rf "$profile"
  else
    # Docker Desktop's file sharing can serve a stale, truncated copy of a
    # file rewritten under the same name, so each read gets a fresh name.
    probe="probe-$$-$attempt-$(date +%s).html"
    cp "$work/$file" "$work/$probe"
    docker run --rm -v "$work:/work" rb-ac-e1-chromium chromium --headless=new --no-sandbox --disable-gpu \
      --disable-dev-shm-usage --virtual-time-budget=15000 --dump-dom "file:///work/$probe" 2>/dev/null > "$out.dom"
    rm -f "$work/$probe"
  fi
  python3 -I -c 'import sys,re,html; d=open(sys.argv[1]).read(); m=re.search(r"<pre id=\"ac-e1-probe\">(.*?)</pre>", d, re.S); t=html.unescape(m.group(1)) if m else ""
if not t.startswith("{"): sys.exit(1)
open(sys.argv[2],"w").write(t)' "$out.dom" "$out" && { rm -f "$out.dom"; exit 0; }
  echo "attempt $attempt: no probe result for $reader $file" >&2
done
rm -f "$out.dom"
exit 1
