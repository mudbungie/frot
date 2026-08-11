#!/bin/sh
# posix-suite.sh — gate the POSIX profile (docs/design/posix.md) against a
# real frot binary. Every assertion cites the §-clause it tests; a pinned
# deviation (D2: bl-34fb) is asserted at its CURRENT behavior so
# the fix cannot land without flipping the test and the doc together.
#
# POSIX sh only — no bashisms, no dependency beyond POSIX utilities and
# Linux's /dev/full (the profile is Linux-scoped, posix.md §7). No network:
# file:// exercises the same envelope path (§6).
#
# Usage: scripts/posix-suite.sh path/to/frot

set -u

FROT=${1:?usage: posix-suite.sh BINARY}
case "$FROT" in /*) ;; *) FROT=$PWD/$FROT ;; esac
[ -x "$FROT" ] || { echo "posix-suite: $FROT is not executable" >&2; exit 2; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

fail=0
ok() { printf 'ok - %s\n' "$1"; }
bad() { printf 'FAIL - %s\n' "$1" >&2; fail=1; }
assert_eq() { # label expected actual
  if [ "$2" = "$3" ]; then ok "$1"; else bad "$1 (expected [$2], got [$3])"; fi
}
assert_empty() { # label file
  if [ ! -s "$2" ]; then ok "$1"; else bad "$1 ($(wc -c <"$2") stray bytes)"; fi
}
assert_has() { # label file fixed-string
  if grep -F -q -e "$3" "$2"; then ok "$1"; else bad "$1 (no [$3])"; fi
}
assert_same() { # label file file
  if cmp -s "$2" "$3"; then ok "$1"; else bad "$1 (outputs differ)"; fi
}
run() { # outfile errfile args... ; exit status lands in RC
  _o=$1 _e=$2; shift 2
  "$FROT" "$@" >"$_o" 2>"$_e"
  RC=$?
}
# §3: every usage error is exit 2, stdout silent, usage line on stderr.
usage_error() { # label args...
  _label=$1; shift
  run "$TMP/u.out" "$TMP/u.err" "$@"
  assert_eq "$_label: exit 2" 2 "$RC"
  assert_empty "$_label: stdout silent" "$TMP/u.out"
  assert_has "$_label: usage on stderr" "$TMP/u.err" "usage: frot"
}

printf '<html><body><p>hello suite</p></body></html>' >"$TMP/ok.html"
printf '<html><head><script src="app.js"></script></head><body><div id="root"></div></body></html>' >"$TMP/spa.html"
OKURL="file://$TMP/ok.html"

# --- §3 utility syntax ------------------------------------------------------

usage_error "no arguments"
usage_error "unknown flag" --bogus "$OKURL" --out text
usage_error "unknown view" "$OKURL" --out nope
usage_error "missing --out" "$OKURL"
usage_error "missing operand" --out text
usage_error "missing option-argument" "$OKURL" --out
usage_error "duplicate flag" "$OKURL" --css --css --out text
usage_error "second operand" "$OKURL" extra --out text
usage_error "--js-errors without --js" "$OKURL" --js-errors --out text
usage_error "-H with file://" "$OKURL" -H "X-A: b" --out text
# XBD 12.2 G10: the first -- ends the options; everything after it is an
# operand, so a flag word after it is the second operand (a usage error), and
# a second -- is an ordinary operand (an unparseable URL, so an error envelope).
usage_error "-- : a flag word after it is an operand" -- "$OKURL" --out text
usage_error "-- : a second operand after it" --out text -- "$OKURL" extra

run "$TMP/eoo.out" "$TMP/eoo.err" --out text -- "$OKURL"
assert_eq "-- : delimiter accepted, exit 0" 0 "$RC"
assert_has "-- : operand after it is the URL" "$TMP/eoo.out" '"status":"ok"'
assert_empty "-- : stderr silent" "$TMP/eoo.err"

run "$TMP/eoo2.out" "$TMP/eoo2.err" "$OKURL" --out text --
assert_eq "bare -- : exit 0" 0 "$RC"
assert_same "bare -- : operand list unchanged" "$TMP/eoo.out" "$TMP/eoo2.out"

run "$TMP/eoo3.out" "$TMP/eoo3.err" --out text -- --
assert_eq "-- : a second -- is an operand (bad URL), exit 1" 1 "$RC"
assert_has "-- : second -- reaches URL parsing" "$TMP/eoo3.out" '"kind":"fetch.url"'

run "$TMP/h.out" "$TMP/h.err" --help
assert_eq "--help: exit 0" 0 "$RC"
assert_has "--help: usage on stdout" "$TMP/h.out" "usage: frot"
assert_empty "--help: stderr silent" "$TMP/h.err"
run "$TMP/h2.out" "$TMP/h2.err" -h
assert_same "-h ≡ --help" "$TMP/h.out" "$TMP/h2.out"

run "$TMP/v.out" "$TMP/v.err" --version
assert_eq "--version: exit 0" 0 "$RC"
assert_has "--version: product on stdout" "$TMP/v.out" "frot "
assert_empty "--version: stderr silent" "$TMP/v.err"
run "$TMP/v2.out" "$TMP/v2.err" -V
assert_same "-V ≡ --version" "$TMP/v.out" "$TMP/v2.out"

# --- §4.2 stdout: one line, one product -------------------------------------

run "$TMP/ok.out" "$TMP/ok.err" "$OKURL" --out text
assert_eq "ok envelope: exit 0" 0 "$RC"
assert_has "ok envelope: status ok" "$TMP/ok.out" '"status":"ok"'
assert_has "ok envelope: versioned" "$TMP/ok.out" '{"frot":"0"'
assert_eq "ok envelope: one line (XBD 3.206)" 1 "$(wc -l <"$TMP/ok.out")"
assert_eq "ok envelope: final byte is newline" "" "$(tail -c 1 "$TMP/ok.out")"
assert_empty "ok envelope: stderr silent (§4.3)" "$TMP/ok.err"

run "$TMP/ok2.out" "$TMP/ok2.err" "$OKURL" --out text
assert_same "determinism: identical runs, identical bytes" "$TMP/ok.out" "$TMP/ok2.out"

run "$TMP/eq1.out" "$TMP/eq1.err" "$OKURL" --out=text
assert_same "--out=V ≡ --out V (declared extension)" "$TMP/ok.out" "$TMP/eq1.out"
run "$TMP/eq2.out" "$TMP/eq2.err" --out text "$OKURL"
assert_same "operand/option interleave (declared extension)" "$TMP/ok.out" "$TMP/eq2.out"

run "$TMP/needs.out" "$TMP/needs.err" "file://$TMP/spa.html" --out text
assert_eq "needs envelope: exit 0" 0 "$RC"
assert_has "needs envelope: status" "$TMP/needs.out" '"status":"needs"'
assert_has "needs envelope: names the need" "$TMP/needs.out" '"needs":["js"]'
assert_empty "needs envelope: stderr silent" "$TMP/needs.err"

run "$TMP/err.out" "$TMP/err.err" "not a url" --out text
assert_eq "error envelope (bad URL): exit 1" 1 "$RC"
assert_has "error envelope: status" "$TMP/err.out" '"status":"error"'
assert_has "error envelope: kind fetch.url" "$TMP/err.out" '"kind":"fetch.url"'
assert_empty "error envelope: stderr silent" "$TMP/err.err"

run "$TMP/err2.out" "$TMP/err2.err" "file://$TMP/absent.html" --out text
assert_eq "error envelope (missing file): exit 1" 1 "$RC"
assert_has "error envelope: kind fetch.file" "$TMP/err2.out" '"kind":"fetch.file"'

# --- §4.1 stdin: never read -------------------------------------------------

"$FROT" "$OKURL" --out text <&- >"$TMP/in1.out" 2>"$TMP/in1.err"
assert_eq "stdin closed: exit 0" 0 "$?"
assert_same "stdin closed: identical product" "$TMP/ok.out" "$TMP/in1.out"
printf 'garbage that must not matter\n' | "$FROT" "$OKURL" --out text >"$TMP/in2.out" 2>"$TMP/in2.err"
assert_eq "stdin data: exit 0" 0 "$?"
assert_same "stdin data: identical product" "$TMP/ok.out" "$TMP/in2.out"

# --- §4.4 D2 (bl-34fb): stdout write failure, pinned current behavior -------

"$FROT" "$OKURL" --out text >&- 2>"$TMP/ebadf.err"
assert_eq "D2 pinned: stdout closed (EBADF), exit by envelope" 0 "$?"
assert_empty "D2 pinned: EBADF stderr silent" "$TMP/ebadf.err"

if [ -e /dev/full ]; then
  "$FROT" "$OKURL" --out text >/dev/full 2>"$TMP/enospc.err"
  assert_eq "D2 pinned: device full (ENOSPC), exit by envelope" 0 "$?"
  assert_empty "D2 pinned: ENOSPC stderr silent" "$TMP/enospc.err"
else
  bad "/dev/full missing — the profile is Linux-scoped (posix.md §7)"
fi

# EPIPE: the reader is long dead before frot writes. The FIFO sequences it —
# frot blocks opening the gate, `true` exits at once, the feeder unblocks
# frot only afterward, so the envelope write hits a widowed pipe. §5.2: frot
# must not die of SIGPIPE (disposition SIG_IGN), so the status is the
# envelope's, not 141.
mkfifo "$TMP/gate.epipe"
{ sleep 1; printf '<p>late</p>' >"$TMP/gate.epipe"; } &
FEEDER=$!
{
  "$FROT" "file://$TMP/gate.epipe" --out text 2>"$TMP/epipe.err"
  echo $? >"$TMP/epipe.status"
} | true
wait "$FEEDER"
read -r epipe_rc <"$TMP/epipe.status"
assert_eq "D2 pinned: EPIPE, exit by envelope (not 141)" 0 "$epipe_rc"
assert_empty "D2 pinned: EPIPE stderr silent" "$TMP/epipe.err"

# --- §5.2 signals: inherited dispositions honored, no partial output --------

signal_test() { # label signame waitstatus
  mkfifo "$TMP/gate.$2"
  "$FROT" "file://$TMP/gate.$2" --out text >"$TMP/sig.out" 2>"$TMP/sig.err" &
  _pid=$!
  sleep 1
  kill -"$2" "$_pid"
  wait "$_pid"
  assert_eq "$1: wait reports 128+n (XCU 2.8.2)" "$3" "$?"
  assert_empty "$1: no partial stdout" "$TMP/sig.out"
  assert_empty "$1: no partial stderr" "$TMP/sig.err"
}
signal_test "SIGTERM (default disposition) while blocked" TERM 143
signal_test "SIGHUP (default disposition) while blocked" HUP 129

# SIGINT reaches this background job as SIG_IGN (XCU 2.11: a non-interactive
# shell runs `&` commands with SIGINT ignored), and frot must not reset an
# inherited ignore — so the process survives the kill. That inheritance is
# the claim; a foreground Ctrl-C (default disposition) terminates like
# SIGTERM above.
mkfifo "$TMP/gate.INTIGN"
"$FROT" "file://$TMP/gate.INTIGN" --out text >"$TMP/sig.out" 2>"$TMP/sig.err" &
INTPID=$!
sleep 1
kill -INT "$INTPID"
sleep 1
if kill -0 "$INTPID" 2>/dev/null; then
  ok "SIGINT inherited as ignored stays ignored (XCU 2.11)"
else
  bad "SIGINT inherited as ignored stays ignored (XCU 2.11)"
fi
kill -TERM "$INTPID"
wait "$INTPID"
assert_eq "SIGINT-ignoring run still terminable by SIGTERM" 143 "$?"

# --- §5.1 lifecycle: EOF at exit proves nothing outlives the call -----------

# Command substitution reads frot's stdout pipe to EOF; it can only return
# when every holder of the write end is gone. Getting the complete envelope
# back here IS the proof that no child/task/reactor survived the exit.
LIFE=$("$FROT" "$OKURL" --out text)
case $LIFE in
*'"status":"ok"'*) ok "lifecycle: EOF at exit, envelope complete" ;;
*) bad "lifecycle: EOF at exit, envelope complete" ;;
esac

# --- §5.3 stateless: no residue ---------------------------------------------

mkdir "$TMP/scratch"
(cd "$TMP/scratch" && HOME="$TMP/scratch" TMPDIR="$TMP/scratch" \
  "$FROT" "$OKURL" --out text >/dev/null 2>&1)
assert_eq "stateless: scratch cwd/HOME/TMPDIR left empty" "" "$(ls -A "$TMP/scratch")"

# ----------------------------------------------------------------------------

if [ "$fail" -ne 0 ]; then
  echo "posix-suite: FAILED against $FROT" >&2
  exit 1
fi
echo "posix-suite: all conformance checks passed against $FROT"
