# The POSIX profile — what frot claims, and does not

This is the **single authoritative statement** of frot's POSIX-facing CLI
contract. Everything else — README, ARCHITECTURE.md, the `src/run.rs` and
`src/cli.rs` doc comments — derives from this file and cites it; none of them
is allowed to state a fact this file does not. The contract is gated by
`scripts/posix-suite.sh` (a `/bin/sh` harness spawning the real binary), run
from `make posix` / `make posix-musl` and CI. Every claim below is either
tested by that suite or by the Rust integration tests in `tests/binary.rs`;
a claim nothing tests does not belong here.

## §1 Pinned edition

The reference is **POSIX.1-2017** (IEEE Std 1003.1-2017; The Open Group Base
Specifications Issue 7, 2018 edition). Clauses are cited as XBD (Base
Definitions), XCU (Shell & Utilities), XSH (System Interfaces). The cited
clauses are unchanged in substance in POSIX.1-2024 (Issue 8); the pin exists
so a citation resolves to exactly one text.

Cited clauses:

- **XBD 12.1** Utility Argument Syntax, **XBD 12.2** Utility Syntax
  Guidelines — argv shape (§3).
- **XCU 1.4** Utility Description Defaults, subsections STDIN, STDOUT,
  STDERR, EXIT STATUS, CONSEQUENCES OF ERRORS — stream discipline and exit
  statuses (§4).
- **XBD 3.206** Line ("a sequence of zero or more non-\<newline\> characters
  plus a terminating \<newline\> character") — the one-line stdout product
  (§4.2).
- **XSH 2.4** Signal Concepts — default-action termination and the SIGPIPE
  disposition (§5.2).
- **XCU 2.8.2** Exit Status for Commands — how the suite observes a
  signal death (`wait` reports 128+n).
- **XCU 2.11** Signals and Error Handling — a non-interactive shell runs
  `&` commands with SIGINT/SIGQUIT ignored; frot honors that inherited
  ignore (§5.2).

## §2 Three separate claims

frot makes three claims of different strength. Conflating them is how a tool
comes to imply certification it does not have, so they are kept apart:

1. **Utility syntax (§3)** — how argv is parsed. Claimed against XBD 12,
   with declared extensions and no declared deviations.
2. **Process I/O and lifecycle (§4–§6)** — streams, exit statuses, signals,
   and what exists after exit. Claimed in full, with one declared
   deviation (D2, §4.4).
3. **Operating-system / ABI portability (§7)** — **not claimed.** frot's
   behavior is tested on two Linux x86_64 targets and asserted nowhere
   else. Nothing in this document is a certification claim; Linux itself is
   not a certified POSIX system.

## §3 Claim 1 — utility syntax (XBD 12.1, 12.2)

The surface is fixed (`src/cli.rs`):

    frot <url> [-H "Name: value"] [--css] [--js] [--js-errors] --out <view>

- **Short options conform.** `-H` (option with a separate option-argument;
  Guidelines 3–7), `-h`, `-V` (argument-less; Guideline 3). The joined form
  `-Hvalue` is not accepted — Guideline 6 asks for the separate form, and
  frot accepts only it.
- **Long options are a declared extension.** `--css`, `--js`, `--js-errors`,
  `--out`, `--header`, `--help`, `--version` and the `--out=VIEW` /
  `--header=H` forms are GNU-style multi-character options, outside XBD 12.2
  (which defines only single-character options). POSIX reserves `--`-prefixed
  words for implementation extension; frot uses them and says so.
- **Operand/option interleaving is a declared extension.** Guideline 9 puts
  all options before operands; frot accepts them in any order
  (`frot --out text URL` ≡ `frot URL --out text`).
- **One operand.** Exactly one, the URL. A second operand is a usage error.
  frot reads nothing from standard input (§4.1), so the Guideline 13 `-`
  operand convention ("`-` means stdin") does not apply: `-` is passed
  through as the URL operand and fails URL parsing like any other non-URL.
- **Errors are loud.** Unknown flag, unknown view, missing operand, missing
  `--out`, a missing option-argument, a duplicated flag, and a rejected
  combination (`--js-errors` without `--js`; `-H` with a `file://` URL) are
  all usage errors: exit 2, diagnostic + usage line on stderr, nothing on
  stdout (§4.3).
- **`--` ends the options (Guideline 10).** The first `--` that is not an
  option-argument is the end-of-options delimiter: every argument after it is
  an operand, including one that begins with `-`. A bare `--` is not itself an
  operand — it leaves the operand list unchanged — and only the first one
  delimits; a later `--` is an ordinary operand. Since everything after it is
  an operand, `frot -- URL --out text` makes `--out` the second operand and so
  a usage error, not a flag.
- **`-h`/`--help` and `-V`/`--version`** write their one product (the usage
  line; `frot <version>`) to **stdout** and exit 0.

## §4 Claim 2a — stream discipline and exit statuses (XCU 1.4)

### §4.1 stdin

Never read, never closed, never checked. frot's behavior is identical
whether stdin is a terminal, a file, or closed.

### §4.2 stdout

stdout carries exactly one product and nothing else:

- An **envelope run** writes exactly **one line** in the XBD 3.206 sense —
  one UTF-8 JSON object, no embedded newline, one terminating `\n` as the
  final byte. The JSON schema itself is the envelope contract
  (`src/envelope.rs`, README "Output views"), not this document's subject.
- **`--help` / `--version`** write their single-line product to stdout.
- A **usage error** writes **zero bytes** to stdout.

The envelope for a `file://` input is byte-for-byte deterministic: two
identical invocations produce identical stdout.

### §4.3 stderr

Diagnostics only (XCU 1.4 STDERR): the usage-error message, nothing else. A
successful run, a `needs` run, and an `error`-envelope run write zero bytes
to stderr. An `error` envelope is a *product* (JSON on stdout), not a
diagnostic.

### §4.4 Exit statuses (authoritative table)

| status | meaning |
|-------:|---------|
| 0 | `ok` or `needs` envelope emitted; also `--help` / `--version` |
| 1 | `error` envelope emitted (still JSON on stdout) |
| 2 | usage error (no envelope; diagnostic on stderr) |
| 128+n | killed by signal n, default disposition (§5.2); no partial output |

No other statuses are used. README's exit-code sentence derives from this
table.

**Declared deviation D2 — stdout write failure is not detected.** If the
envelope write fails (EPIPE: reader gone; EBADF: stdout closed; ENOSPC:
device full), frot ignores the failure and exits by envelope status — a lost
product can exit 0, silently. XCU 1.4 (CONSEQUENCES OF ERRORS) and frot's
own honest-signals principle both say it should not. Fixing it changes the
exit-status surface and is tracked as **bl-34fb**; until that lands, the
suite pins the current behavior: exit status unchanged, no crash, no signal
death, stderr silent.

## §5 Claim 2b — lifecycle

### §5.1 Foreground, blocking, complete

One invocation is one foreground process. When `wait()` reports frot's exit,
the product is complete: the suite reads frot's stdout through a pipe to
EOF, and EOF at exit is the proof that no child, thread, task, or reactor
still holds the descriptor. The internal tokio runtime is per-invocation and
`block_on`-driven (ARCHITECTURE.md, `bl-abca`); no work escapes the call.
frot never forks, never daemonizes, never detaches.

### §5.2 Signals (XSH 2.4)

frot installs no signal handlers and alters no disposition except SIGPIPE:

- **Default dispositions terminate.** A signal whose disposition is default
  at delivery (the suite uses SIGTERM and SIGHUP) causes abnormal
  termination, observed by the shell as 128+n (143, 129). No partial
  envelope is emitted: stdout and stderr carry zero bytes from a killed run
  (the envelope is written once, whole, at the end).
- **Inherited ignores stay ignored.** frot never resets a disposition it
  inherited: a background job under a non-interactive shell receives SIGINT
  as SIG_IGN (XCU 2.11) and frot survives the kill; a foreground Ctrl-C
  (default disposition) terminates it like SIGTERM.
- **SIGPIPE**: ignored (the Rust runtime sets `SIG_IGN` before `main`), so
  frot never terminates with status 141; a broken pipe surfaces as a write
  error instead — currently swallowed, see D2.

No cleanup-on-signal is claimed or needed: there is nothing to clean (§5.3).

### §5.3 Stateless — no residue

An invocation creates no files: nothing in the working directory, `$HOME`,
or `$TMPDIR`; no lock files, no caches, no state that a second invocation
could observe (VISION.md principle 1). The suite runs frot inside an empty
scratch tree with `HOME` and `TMPDIR` pointed at it and asserts the tree is
still empty afterward.

## §6 What the suite exercises

`scripts/posix-suite.sh BINARY` asserts every §3–§5 claim against the real
binary: envelope runs (`ok` via a local file, `needs` via an SPA shell,
`error` via an unparseable URL and a missing file), every §3 usage error,
help/version, `--out=` and interleaving equivalence, determinism, closed /
full / broken stdout (D2 as pinned), the `--` end-of-options delimiter,
closed stdin, SIGTERM/SIGHUP death and the SIGINT inherited-ignore while
blocked on a FIFO read, the pipe-EOF lifecycle proof, and the
empty-scratch-tree residue check. It is
POSIX sh (`/bin/sh`, no bashisms) with no dependency beyond POSIX utilities
plus Linux's `/dev/full` (§7 makes the suite Linux-scoped anyway). Network
behavior needs no server: `file://` exercises the same envelope path.

## §7 Claim 3 — portability: not claimed

frot's contract is asserted **only** on the targets its gates actually run:

- `x86_64-unknown-linux-gnu` — the development and CI build (`make posix`).
- `x86_64-unknown-linux-musl` — the shipped release artifact
  (`make posix-musl`; the release pipeline runs the suite against the exact
  stripped binary it uploads).

That is the whole claim. frot is **not** POSIX-certified, does not claim
UNIX branding, and asserts nothing about macOS, the BSDs, illumos, or any
other system — the code may well work there, but nothing gates it, so this
document does not say it. Under-claiming precisely beats over-claiming
vaguely.
