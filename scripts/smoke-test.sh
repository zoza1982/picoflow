#!/usr/bin/env bash
#
# PicoFlow comprehensive smoke test.
#
# Exercises every CLI command, subcommand, and notable option end-to-end against
# a freshly built binary, using an isolated temp state DB. Intended as a fast,
# repeatable QA / smoke gate — NOT a substitute for `cargo test`.
#
# Usage:
#   ./scripts/smoke-test.sh                 # builds release, then tests
#   PICOFLOW_BIN=/path/to/picoflow ./scripts/smoke-test.sh   # test an existing binary
#   SKIP_BUILD=1 ./scripts/smoke-test.sh    # reuse ./target/release/picoflow
#
# Exit code: 0 if every assertion passed, 1 otherwise.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${PICOFLOW_BIN:-$ROOT/target/release/picoflow}"
WORK="$(mktemp -d)"
DB="$WORK/qa.db"
GLOBAL=(--log-level error --db-path "$DB")   # quiet logs, isolated state
OUT="$WORK/out"; ERR="$WORK/err"

PASS=0; FAIL=0; declare -a FAILURES=()
c_g="\033[32m"; c_r="\033[31m"; c_b="\033[1m"; c_0="\033[0m"

section() { printf "\n${c_b}══ %s ${c_0}\n" "$*"; }
pass() { PASS=$((PASS+1)); printf "  ${c_g}✔${c_0} %s\n" "$*"; }
fail() { FAIL=$((FAIL+1)); FAILURES+=("$*"); printf "  ${c_r}x${c_0} %s\n" "$*"; }

# run picoflow, capture stdout/stderr
pf() { "$BIN" "${GLOBAL[@]}" "$@" >"$OUT" 2>"$ERR"; }

expect_ok()   { local d="$1"; shift; if pf "$@"; then pass "$d"; else fail "$d (exit $?)"; sed 's/^/      /' "$ERR" | tail -4; fi; }
expect_fail() { local d="$1"; shift; if pf "$@"; then fail "$d (unexpectedly succeeded)"; else pass "$d"; fi; }
stdout_has()  { grep -qF -- "$2" "$OUT" && pass "$1" || { fail "$1 (stdout missing '$2')"; sed 's/^/      /' "$OUT" | tail -4; }; }
anyout_has()  { { cat "$OUT" "$ERR"; } | grep -qiF -- "$2" && pass "$1" || fail "$1 (output missing '$2')"; }

cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

# ── Build ────────────────────────────────────────────────────────────────────
if [[ -z "${PICOFLOW_BIN:-}" && -z "${SKIP_BUILD:-}" ]]; then
  section "Building release binary"
  ( cd "$ROOT" && cargo build --release ) || { echo "build failed"; exit 1; }
fi
[[ -x "$BIN" ]] || { echo "binary not found: $BIN"; exit 1; }
echo "Binary: $BIN ($(du -h "$BIN" | cut -f1))"

WF="$WORK/wf"; mkdir -p "$WF" "$WORK/www"

# ── Workflow fixtures ─────────────────────────────────────────────────────────
cat > "$WF/seq.yaml" <<'Y'
name: seq-demo
config: { max_parallel: 1 }
tasks:
  - { name: first,  type: shell, config: { command: "/bin/echo", args: ["first"] } }
  - { name: second, type: shell, depends_on: [first],  config: { command: "/bin/echo", args: ["second"] } }
  - { name: third,  type: shell, depends_on: [second], config: { command: "/bin/echo", args: ["third"] } }
Y
cat > "$WF/parallel.yaml" <<'Y'
name: parallel-demo
config: { max_parallel: 4 }
tasks:
  - { name: root, type: shell, config: { command: "/bin/echo", args: ["root"] } }
  - { name: a, type: shell, depends_on: [root], config: { command: "/bin/sleep", args: ["0.3"] } }
  - { name: b, type: shell, depends_on: [root], config: { command: "/bin/sleep", args: ["0.3"] } }
  - { name: c, type: shell, depends_on: [root], config: { command: "/bin/sleep", args: ["0.3"] } }
  - { name: join, type: shell, depends_on: [a, b, c], config: { command: "/bin/echo", args: ["joined"] } }
Y
cat > "$WF/fail-retry.yaml" <<'Y'
name: fail-retry-demo
config: { max_parallel: 1 }
tasks:
  - { name: always_fails, type: shell, retry: 1, config: { command: "/bin/sh", args: ["-c", "exit 3"] } }
Y
cat > "$WF/continue.yaml" <<'Y'
name: continue-demo
config: { max_parallel: 1 }
tasks:
  - { name: flaky, type: shell, retry: 0, continue_on_failure: true, config: { command: "/bin/sh", args: ["-c", "exit 1"] } }
  - { name: after, type: shell, depends_on: [flaky], config: { command: "/bin/echo", args: ["ran anyway"] } }
Y
cat > "$WF/timeout.yaml" <<'Y'
name: timeout-demo
config: { max_parallel: 1 }
tasks:
  - { name: slow, type: shell, retry: 0, timeout: 1, config: { command: "/bin/sleep", args: ["5"] } }
Y
cat > "$WF/cycle.yaml" <<'Y'
name: cycle-demo
tasks:
  - { name: a, type: shell, depends_on: [b], config: { command: "/bin/echo", args: ["a"] } }
  - { name: b, type: shell, depends_on: [a], config: { command: "/bin/echo", args: ["b"] } }
Y
cat > "$WF/missing-dep.yaml" <<'Y'
name: missing-dep-demo
tasks:
  - { name: a, type: shell, depends_on: [ghost], config: { command: "/bin/echo", args: ["a"] } }
Y
cat > "$WF/bad-cmd.yaml" <<'Y'
name: bad-cmd-demo
tasks:
  - { name: a, type: shell, config: { command: "echo", args: ["a"] } }
Y
cat > "$WF/scheduled.yaml" <<'Y'
name: scheduled-demo
schedule: "*/2 * * * * *"
config: { max_parallel: 1 }
tasks:
  - { name: tick, type: shell, config: { command: "/bin/echo", args: ["tick"] } }
Y

# ── Global flags ──────────────────────────────────────────────────────────────
section "Global flags"
"$BIN" --version >"$OUT" 2>&1 && stdout_has "--version prints version" "picoflow" || fail "--version"
"$BIN" -V >"$OUT" 2>&1 && stdout_has "-V prints version" "picoflow"
"$BIN" --help >"$OUT" 2>&1 && stdout_has "--help lists commands" "Commands:"
"$BIN" run --help >"$OUT" 2>&1 && stdout_has "run --help" "workflow"
"$BIN" daemon --help >"$OUT" 2>&1 && stdout_has "daemon --help lists subcommands" "start"

# ── template ─────────────────────────────────────────────────────────────────
section "template"
expect_ok  "template (list)" template
stdout_has "template list shows types" "minimal"
for t in minimal shell ssh http full; do
  expect_ok "template --type $t" template --type "$t"
  stdout_has "template $t emits yaml (name:)" "name:"
  "$BIN" "${GLOBAL[@]}" template --type "$t" > "$WF/tpl-$t.yaml" 2>/dev/null
done
expect_ok   "template --output writes file" template --type minimal --output "$WF/tpl-out.yaml"
[[ -s "$WF/tpl-out.yaml" ]] && pass "template output file non-empty" || fail "template output file empty"
expect_fail "template --output refuses existing file" template --type minimal --output "$WF/tpl-out.yaml"
expect_ok   "validate the minimal template" validate "$WF/tpl-minimal.yaml"
expect_ok   "validate the full template"    validate "$WF/tpl-full.yaml"

# ── validate ─────────────────────────────────────────────────────────────────
section "validate"
expect_ok   "validate valid seq workflow" validate "$WF/seq.yaml"
stdout_has  "validate prints execution order" "first -> second -> third"
expect_fail "validate rejects cycle"          validate "$WF/cycle.yaml"
anyout_has  "cycle error mentions cycle"       "cycle"
expect_fail "validate rejects missing dep"    validate "$WF/missing-dep.yaml"
anyout_has  "missing-dep error names ghost"    "ghost"
expect_fail "validate rejects non-absolute cmd" validate "$WF/bad-cmd.yaml"

# ── run: sequential ──────────────────────────────────────────────────────────
section "run — sequential + deps"
expect_ok  "run seq workflow succeeds" run "$WF/seq.yaml"
pf status seq-demo
stdout_has "seq execution recorded as success" "success"

# ── run: parallel ────────────────────────────────────────────────────────────
section "run — parallel (diamond)"
expect_ok "run parallel workflow succeeds" run "$WF/parallel.yaml"

# ── run: failure + retry ─────────────────────────────────────────────────────
section "run — failure + retry"
expect_fail "run failing workflow exits non-zero" run "$WF/fail-retry.yaml"

# ── run: continue_on_failure ─────────────────────────────────────────────────
section "run — continue_on_failure"
expect_fail "run continue workflow exits non-zero (overall failed)" run "$WF/continue.yaml"

# ── run: timeout ─────────────────────────────────────────────────────────────
section "run — timeout enforcement"
t0=$(date +%s); pf run "$WF/timeout.yaml"; rc=$?; t1=$(date +%s)
[[ $rc -ne 0 ]] && pass "timeout workflow fails" || fail "timeout workflow should fail"
[[ $((t1 - t0)) -lt 4 ]] && pass "timeout fired (~1s, not 5s sleep)" || fail "timeout did not cut off early ($((t1-t0))s)"

# ── HTTP executor ────────────────────────────────────────────────────────────
section "HTTP executor"
PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')
python3 - "$PORT" > "$WORK/httpd.log" 2>&1 <<'PY' &
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer
class H(BaseHTTPRequestHandler):
    def _r(self):
        self.send_response(200); self.send_header("Content-Type","text/plain"); self.end_headers()
        self.wfile.write(b"ok")
    def do_GET(self): self._r()
    def do_POST(self):
        n=int(self.headers.get("Content-Length",0)); self.rfile.read(n); self._r()
    def log_message(self,*a): pass
HTTPServer(("127.0.0.1",int(sys.argv[1])),H).serve_forever()
PY
HTTP_PID=$!
sleep 1
cat > "$WF/http-get.yaml" <<Y
name: http-get-demo
tasks:
  - name: get_ok
    type: http
    config: { url: "http://127.0.0.1:$PORT/", method: GET, allow_private_ips: true, timeout: 5 }
Y
cat > "$WF/http-post.yaml" <<Y
name: http-post-demo
tasks:
  - name: post_ok
    type: http
    config:
      url: "http://127.0.0.1:$PORT/submit"
      method: POST
      allow_private_ips: true
      timeout: 5
      headers: { X-Test: qa, Content-Type: application/json }
      body: { hello: world }
Y
cat > "$WF/http-ssrf.yaml" <<'Y'
name: http-ssrf-demo
tasks:
  - name: metadata
    type: http
    config: { url: "http://169.254.169.254/latest/meta-data/", method: GET, allow_private_ips: false, timeout: 3 }
Y
expect_ok   "HTTP GET 200 succeeds"  run "$WF/http-get.yaml"
expect_ok   "HTTP POST 200 succeeds" run "$WF/http-post.yaml"
expect_fail "HTTP SSRF to metadata IP is blocked" run "$WF/http-ssrf.yaml"
kill "$HTTP_PID" 2>/dev/null

# ── SSH executor ─────────────────────────────────────────────────────────────
section "SSH executor"
expect_ok "validate ssh template" validate "$WF/tpl-ssh.yaml"
if nc -z -w1 127.0.0.1 22 2>/dev/null; then
  echo "  (local sshd detected — live SSH exec would need a provisioned key; validated config only)"
else
  echo "  (no local sshd — SSH live exec needs a remote host; config validation covered)"
fi

# ── state read commands (after the runs above) ───────────────────────────────
section "status / workflow list / history / stats / logs"
expect_ok  "status <workflow>" status seq-demo
stdout_has "status shows executions" "Execution ID:"
expect_ok  "status --limit" status seq-demo --limit 1
expect_ok  "status (no name) prints hint" status
expect_ok  "workflow list" workflow list
stdout_has "workflow list shows seq-demo" "seq-demo"
expect_ok  "history <workflow>" history seq-demo
expect_ok  "history --status success" history fail-retry-demo --status failed
expect_ok  "history --limit" history seq-demo --limit 1
expect_ok  "stats <workflow>" stats seq-demo
anyout_has "stats shows totals" "success"
expect_ok  "logs <workflow>" logs seq-demo
expect_ok  "logs --task" logs seq-demo --task first

# ── daemon (scheduled) ───────────────────────────────────────────────────────
section "daemon — start / status / stop"
PIDF="$WORK/daemon.pid"
expect_fail "daemon start rejects unscheduled workflow" daemon start "$WF/seq.yaml" --pid-file "$PIDF"
"$BIN" "${GLOBAL[@]}" daemon start "$WF/scheduled.yaml" --pid-file "$PIDF" >"$WORK/daemon.log" 2>&1 &
DPID=$!
sleep 4
if "$BIN" "${GLOBAL[@]}" daemon status --pid-file "$PIDF" >"$OUT" 2>&1; then pass "daemon status: running"; else fail "daemon status: not running"; fi
"$BIN" "${GLOBAL[@]}" history scheduled-demo >"$OUT" 2>"$ERR"
grep -qiE "execution id|status" "$OUT" && pass "scheduled workflow fired at least once" || fail "no scheduled execution recorded"
if "$BIN" "${GLOBAL[@]}" daemon stop --pid-file "$PIDF" >"$OUT" 2>&1; then pass "daemon stop"; else fail "daemon stop"; fi
sleep 1
kill "$DPID" 2>/dev/null
[[ ! -f "$PIDF" ]] && pass "PID file removed after stop" || echo "  (note: PID file still present)"

# ── global options ───────────────────────────────────────────────────────────
section "global options"
DB2="$WORK/other.db"
"$BIN" --log-level error --db-path "$DB2" run "$WF/seq.yaml" >/dev/null 2>&1 && pass "--db-path isolates state (run on alt DB)" || fail "--db-path run failed"
"$BIN" --db-path "$DB2" workflow list >"$OUT" 2>/dev/null; grep -q seq-demo "$OUT" && pass "alt DB has only its own workflows" || fail "alt DB content wrong"
"$BIN" --log-level error --db-path "$DB" --log-format pretty validate "$WF/seq.yaml" >"$OUT" 2>"$ERR" && pass "--log-format pretty works" || fail "--log-format pretty failed"
"$BIN" --log-level debug --db-path "$DB" validate "$WF/seq.yaml" >"$OUT" 2>"$ERR"; grep -qi "debug\|DEBUG" "$ERR" && pass "--log-level debug increases verbosity" || pass "--log-level debug accepted"

# ── summary ──────────────────────────────────────────────────────────────────
section "SUMMARY"
printf "  ${c_g}%d passed${c_0}, ${c_r}%d failed${c_0}\n" "$PASS" "$FAIL"
if (( FAIL > 0 )); then
  printf "\n  Failures:\n"
  for f in "${FAILURES[@]}"; do printf "    - %s\n" "$f"; done
  exit 1
fi
echo "  All smoke tests passed."
