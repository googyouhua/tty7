#!/usr/bin/env bash
# Capture a live "connect froze the UI" scene for tty7.
#
#   ./capture-connect-hang.sh [PID] [OUTDIR]
#
# With no PID it finds the newest tty7-app window process.
# Everything lands in OUTDIR (default /tmp/opencode/connect-hang-<epoch>).
# Read the section headers; the last one prints the verdict hints.

set -uo pipefail
PID="${1:-}"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="${2:-/tmp/opencode/connect-hang-$STAMP}"
TTY7_BIN="${TTY7_BIN:-$(command -v tty7 || echo ./target/debug/tty7)}"

if [[ -z "$PID" ]]; then
  PID="$(ps -eo pid,lstart,comm --sort=start_time | awk '$3=="tty7-app"{p=$1} END{print p}')"
fi
if [[ -z "$PID" || ! -d "/proc/$PID" ]]; then
  echo "no tty7-app process found; pass a PID" >&2
  exit 1
fi

mkdir -p "$OUT"
say() { printf '\n===== %s =====\n' "$*" | tee -a "$OUT/summary.txt" >/dev/null; printf '\n===== %s =====\n' "$*"; }

say "process $PID $(tr '\0' ' ' < /proc/$PID/cmdline 2>/dev/null)"
ps -o pid,ppid,etime,stat,pcpu,rss,args -p "$PID" 2>&1 | tee "$OUT/process.txt"

say "threads (comm / wchan / syscall)"
for t in /proc/$PID/task/*; do
  tid="$(basename "$t")"
  printf '%-8s %-18s %-28s %s\n' "$tid" \
    "$(cat "$t/comm" 2>/dev/null)" \
    "$(cat "$t/wchan" 2>/dev/null)" \
    "$(cut -d' ' -f1 "$t/syscall" 2>/dev/null)"
done | tee "$OUT/threads.txt"

say "kernel stacks (needs root)"
for t in /proc/$PID/task/*; do
  echo "--- $(basename "$t") ---"
  cat "$t/stack" 2>&1
done | tee "$OUT/kernel-stacks.txt"

if command -v gdb >/dev/null 2>&1; then
  say "userspace stacks (gdb, best effort)"
  timeout 60 gdb -p "$PID" -batch -ex 'thread apply all bt 12' 2>&1 \
    | tee "$OUT/gdb-stacks.txt" | tail -40
else
  say "gdb missing — userspace stacks unavailable (install gdb for the decisive view)"
fi

say "sockets held by the app (control / ssh links)"
ss -xap 2>/dev/null | grep -F "pid=$PID," | tee "$OUT/app-sockets.txt"
ss -tnp 2>/dev/null | grep -F "pid=$PID," | tee -a "$OUT/app-sockets.txt"

say "ssh children still alive under the app"
pgrep -a -P "$PID" 2>/dev/null | tee "$OUT/ssh-children.txt"
ps -eo pid,ppid,etime,stat,args | awk -v p="$PID" '$2==p' | tee -a "$OUT/ssh-children.txt"

say "is the local daemon itself stuck? (this is the C-vs-A discriminator)"
for cmd in status ls; do
  s=$(date +%s%N)
  timeout 15 "$TTY7_BIN" "$cmd" >"$OUT/daemon-$cmd.txt" 2>&1
  rc=$?
  printf 'tty7 %s -> exit=%s %sms\n' "$cmd" "$rc" "$(( ($(date +%s%N) - s) / 1000000 ))"
  [[ $rc -ne 0 ]] && echo "  ^ daemon-side hang: the server, not just the client, is not answering"
done | tee -a "$OUT/daemon-verdict.txt"
sed -n '1,20p' "$OUT/daemon-status.txt" 2>/dev/null

say "persisted client state"
for f in "${TTY7_CONFIG_DIR:-$HOME/.config/tty7}"/{views.json,machine.json,daemon.pid}; do
  [[ -f "$f" ]] && cp "$f" "$OUT/" 2>/dev/null && echo "copied $f"
done
python3 - "$HOME/.config/tty7/views.json" <<'PY' 2>/dev/null | tee "$OUT/views-summary.txt"
import json,sys
try:
    d=json.load(open(sys.argv[1]))
except Exception as e:
    print("views.json unreadable:",e); raise SystemExit
for v in d.get("views",[]):
    w=v.get("window",{})
    x,y=w.get("x",0),w.get("y",0)
    flag="  <-- negative/off-screen" if x<0 or y<0 else ""
    print(f"{v.get('label')}: x={x} y={y} {w.get('width')}x{w.get('height')} open={v.get('open')}{flag}")
print("active:", d.get("active"))
PY

say "verdict hints"
cat <<'TXT'
daemon answers in ms + app thread parked in read  -> client-side silence (the missing deadline)
daemon also hangs                                 -> server-side stall (lock/thread), client has no timeout to give up
app thread parked in futex with no socket read    -> a UI gate/sheet is holding the connect, not the network
gdb shows the UI thread inside a blocking call    -> that call is the freeze; send the bt
TXT

echo
echo "scene saved to: $OUT"
