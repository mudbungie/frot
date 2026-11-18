#!/usr/bin/env bash
# leak (from yog) rules — the TABLE, and only the table. `scripts/leak-scan.sh` is the
# mechanism that runs it; this file is what counts as a leak. Split out under
# bl-167d so both halves stay inside the 300-line cap and so "what may not be
# committed" reads as one list.
#
# Sourced, never executed.
#
#   PATTERN — ERE matched with `grep -o`, so a hit is the offending text.
#   EXCEPT  — dropped if the matched text matches this ERE, ANCHORED AT THE
#             START of the match (there is nowhere else to anchor: the hit line
#             is `path:line:match`, so `^` would mean the path, while `$` does
#             mean the end of the match, because the match ends the line).
#             Write a leading `.*` where you mean "contains".
#   WHY     — printed with the finding; it must say what to DO.
#
# ONE INVARIANT GOVERNS THIS FILE: **no pattern here may match this file.**
# The scanner scans itself and this table — bl-167d deleted the exemption that
# made the file the rules live in the one place a leak could not be seen — so a
# rule whose literal text is also an instance of itself would flag the table
# forever. Exactly one ever was: PuTTY's key banner, the only bare literal with
# nothing after it. It is written `Fil[e]` for that reason, the same idiom as
# `ps | grep '[s]shd'`. Every other pattern is self-immune already, because its
# literal prefix is followed by a bracket expression or an alternation bar.
#
# There is no per-rule path exemption and no allowlist. There was one — a SKIP
# prefix for `docs/drive-logs/`, whose files were operator-home paths by the
# hundred — and bl-244f burned those logs rather than keep the carve-out they
# justified. Every rule applies to every tracked file. Add an escape back only
# when a rule is wrong, and fix the rule instead if you can.

# **The table is a `case`, not a `declare -A` map** (bl-1015). Associative
# arrays need bash 4; macOS ships bash 3.2 and, the licence having changed under
# it, always will — so a subscripted table made this gate, and every test of it,
# Linux-only on a repo whose CI builds both platforms.
#
# The shape it takes is load-bearing twice over. A rule's three fields stay
# **together**, under its name and its own commentary, which is what makes this
# a table rather than three lists to keep in step. And the name on the left of
# every `=` is `PATTERN`/`EXCEPT`/`WHY`, never the rule's own — which is what
# keeps the file self-immune under the invariant above: spelled the other way
# (`WHY_vendor_token=`) the *variable name* is itself an instance of
# `credential-assignment`, and the table flags itself forever.
#
#   rule_fields home-path   ->  $PATTERN, $EXCEPT, $WHY for that rule
#
# A rule with no EXCEPT leaves it empty, and so does a name no arm matches;
# there is no error case, because RULES below is the only source of names.
rule_fields() {
  PATTERN='' EXCEPT='' WHY=''
  case "$1" in

  private-key)
    PATTERN='-----BEGIN( [A-Z0-9]+)* PRIVATE KEY-----|PuTTY-User-Key-Fil[e]'
    WHY='a private key. Remove it, then rotate it — it is burned.' ;;

  vendor-token)
    PATTERN='sk-ant-[A-Za-z0-9_-]{16,}|sk-(proj-)?[A-Za-z0-9]{32,}|gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|(AKIA|ASIA)[0-9A-Z]{16}|AIza[0-9A-Za-z_-]{35}|xox[abprs]-[A-Za-z0-9-]{10,}|glpat-[A-Za-z0-9_-]{20}|eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}'
    WHY='a live API token. Remove it, then revoke it at the vendor.' ;;

  credential-assignment)
    PATTERN="(pass(word|wd|phrase)?|secret|token|api[_-]?key|apikey|credential|authorization)['\"]?[[:space:]]*[:=][[:space:]]*['\"][^'\"]{8,}['\"]"
    # Two escapes. The first is the declared placeholder vocabulary. The second is
    # structural and does the real work: a multi-word ALL-ALPHABETIC value is
    # prose, not a secret — `"credential":"not required"` is a status field, and
    # `bz --list-providers --json` emits four of them (src/config_edit/brazen/
    # providers/tests.rs). A secret that survives it must carry a digit, a symbol,
    # or no space at all, which every real one does.
    EXCEPT=".*(REDACTED|redacted|<[^>]*>|\\\$\{|\\\$[A-Z_]+|example|placeholder|dummy|fake|changeme|xxxx|TODO|\.\.\.|test|invalid)|.*['\"][A-Za-z]+( [A-Za-z]+)+['\"]\$"
    WHY='a credential assigned a real-looking value. Use a placeholder ("<redacted>", "${VAR}", "example") if it is illustrative.' ;;

  # THE RULE THAT WOULD HAVE STOPPED frot 0.0.0 AND 0.0.1 (bl-59dc): both shipped
  # a field trial's egress address in the design docs and were yanked for it.
  # The trailing `(\.[0-9]+)*` exists to be thrown away: a dotted run of five or
  # more parts is a section number (ES2026 §27.1.3.3.4) or a version, never an
  # address, and only by consuming the whole run can the EXCEPT see its length.
  ipv4-routable)
    PATTERN='\b((25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])(\.[0-9]+)*\b'
    # Loopback, the unspecified and broadcast addresses, and the three RFC5737
    # documentation ranges. Everything else — RFC1918 included — is network
    # topology, and internal topology is exactly the kind of thing this scan is
    # for. The one literal is SVG path data in a vendored bundle
    # (tests/fixtures/js/vite-react-tailwind.bundle.js), which nobody may edit.
    EXCEPT='127\.|0\.0\.0\.0|255\.255\.255\.255|192\.0\.2\.|198\.51\.100\.|203\.0\.113\.|([0-9]+\.){4}[0-9]|1\.5\.75\.75$'
    WHY='a routable IP address. Use a documentation range (192.0.2.x, 198.51.100.x, 203.0.113.x) or loopback.' ;;

  # Two shapes. The full 8-group form, whatever its prefix; and the COMPRESSED
  # form (`::`) in global-unicast space only (first hex digit 2 or 3) — the form a
  # public address is actually written in. A compressed address outside that
  # prefix cannot be told from Rust path syntax (`feed::cafe`), and a rule that
  # fires on paths gets turned off; an identifier cannot start with a digit, so
  # the leading `[23]` is what makes this half safe.
  ipv6-address)
    PATTERN='\b[0-9a-fA-F]{1,4}(:[0-9a-fA-F]{1,4}){7}\b|\b[23][0-9a-fA-F]{3}:([0-9a-fA-F]{1,4}:){0,6}:([0-9a-fA-F]{1,4}(:[0-9a-fA-F]{1,4}){0,6})?'
    EXCEPT='2001:0*[dD][bB]8:'
    WHY='an IPv6 address. Use the 2001:db8:: documentation prefix (RFC 3849).' ;;

  # WHERE an address is, said two more ways (bl-59dc). The leaked sentence was
  # an address, then its autonomous-system number, its carrier and its city: an
  # ASN names the ISP behind a measurement as surely as the address does, and a
  # coordinate pair names the street. RFC 5398 reserves 64496-64511 for
  # documentation. A place or carrier NAME in prose is beyond any regex — see
  # the known limits in leak-scan.sh.
  asn)
    PATTERN='\bAS[0-9]{2,}\b'
    EXCEPT='AS(6449[6-9]|645(0[0-9]|1[01]))$'
    WHY='an autonomous-system number — it names the network a measurement was taken from. Drop it, or use a documentation ASN (AS64496-AS64511).' ;;

  geo-coordinate)
    # `[^0-9A-Za-z,]{0,3}` is a degree sign and a space, spelled as bytes so the
    # rule reads the same under LC_ALL=C.
    PATTERN="\b(lat(itude)?|lo?ng(itude)?|lon)['\"]?[[:space:]]*[:=][[:space:]]*['\"]?-?[0-9]{1,3}\.[0-9]{2,}|[0-9]{1,3}\.[0-9]{2,}[^0-9A-Za-z,]{0,3}[NS],?[[:space:]]+[0-9]{1,3}\.[0-9]{2,}[^0-9A-Za-z,]{0,3}[EW]\b"
    # Null Island: the origin is nobody's location.
    EXCEPT=".*[:=][[:space:]]*['\"]?-?0+\.0+\$|0+\.0+[^0-9A-Za-z,]{0,3}[NS],?[[:space:]]+0+\.0+[^0-9A-Za-z,]{0,3}[EW]\$"
    WHY='a geographic coordinate. A location is an identity; use 0.00 (Null Island) if it is illustrative.' ;;

  mac-address)
    PATTERN='\b([0-9a-fA-F]{2}:){5}[0-9a-fA-F]{2}\b'
    WHY='a MAC address — a permanent hardware identifier for one machine.' ;;

  # Every home root on every platform, NOT the scanning account's own `$HOME`
  # (bl-167d). The old rule was built from `$HOME` at scan time, which read as
  # identity-free and was in fact runner-dependent: CI runs as `runner`, so it
  # could not see the author's home at all, and no box but the author's could
  # see the author's paths. The rule that holds everywhere is the inverse — an absolute
  # path under ANY home root is a leak unless its account name is one of the
  # house's three synthetic roots (`/home/u`, `/home/op`, `/home/x`, AGENTS.md).
  #
  # `(^|[^A-Za-z0-9._/-])` is load-bearing: `/home` also occurs as an interior
  # path segment with a wholly different meaning — `<world>/walls/home/brazen`
  # is a wall leaf (DESIGN §3.1), not anybody's home — so the rule fires only
  # where `/home/` STARTS an absolute path. The trailing boundary is the other
  # half: a synthetic root must never match inside a longer account name.

  home-path)
    PATTERN='(^|[^A-Za-z0-9._/-])((/home|/Users)/|[A-Za-z]:[\\/][Uu]sers[\\/])[A-Za-z0-9._-]+([^A-Za-z0-9._-]|$)'
    EXCEPT='.?((/home|/Users)/|[A-Za-z]:[\\/][Uu]sers[\\/])(u|op|x)([^A-Za-z0-9._-]|$)'
    WHY='an absolute path under a home directory — it names an account, and it is a portability bug besides. Use a synthetic root (/home/u).' ;;

  # An address is an identity, and one pasted from a config, a transcript or a
  # stack trace names a person who did not publish it. The escape is the reserved
  # documentation and test space (RFC2606/RFC6761): example.com/org/net and the
  # `.example`, `.invalid`, `.test`, `.local`, `.localhost` TLDs, anchored at the END of the
  # match, so a domain that merely BEGINS with a reserved label cannot borrow
  # the escape.
  #
  # The one real address in this tree is the MIT copyright holder's, in LICENSE.
  # That is a DECLARATION of authorship on a crate meant to be published, not a
  # disclosure, and it is written here rather than exempting the LICENSE path:
  # an exception you can read in the rule is reviewable, a path exemption is a
  # hole. It is the only literal identity in this file, and it is already public.

  personal-email)
    PATTERN='[A-Za-z0-9._%+-]+@[A-Za-z0-9._-]+\.[A-Za-z]{2,}'
    EXCEPT='.*@([A-Za-z0-9-]+\.)*(example\.(com|org|net)|example|invalid|test|local|localhost|localdomain)$|mudbungie@gmail\.com$'
    WHY='an email address. Use a documentation address (u@example.com) or a reserved TLD (t@t.local, t@test.invalid).' ;;

  # Pasted conversation. A transcript is content somebody said, and the shape it
  # arrives in is a speaker label at the head of a line — bare, quoted, bulleted
  # or bolded. The vendor JSON envelope is deliberately NOT here: yog PARSES
  # transcripts, so `"role":"assistant"` occurs by the hundred in its own tests,
  # and a rule that fires there would be turned off within the day. The envelope
  # is covered where it is unambiguous instead — see session-artifact.

  quoted-dialogue)
    PATTERN='^[[:space:]>*_#-]*(Human|Assistant|Claude|ChatGPT|Copilot|Gemini|Codex)[*_]{0,2}[[:space:]]*:[[:space:]]+[^[:space:]]'
    WHY='transcribed dialogue attributed to a speaker. A conversation is private content — cite the conclusion, do not paste the exchange.' ;;

  # (bl-9fbe below is the seat's ball, where the finding was filed; this port
  # landed as bl-1e67. The rule text is byte-identical across the four tables.)
  # THE URL IS THE FORM THAT ACTUALLY LEAKED (bl-9fbe; operator ruling
  # 2026-08-30, "ban them, no reason to allow it"). The one published instance
  # was not a bare id in a file — it was a vendor code-session URL in a
  # pull-request BODY, put there by harness convention and permanent because
  # the forge serves `refs/pull/<n>/head` forever. Two alternatives close it
  # and each owns its own fixture line, so neither can hide behind the other:
  # `session` joins the resource-prefix list, which catches the bare id
  # wherever it is written, and the code-session URL path shape catches an id
  # under that path whatever its prefix — the half a prefix list can never be
  # complete over. Both stay self-immune the way the header above requires: the
  # prefix is followed here by `)` and the path by a bracket expression.

  session-artifact)
    PATTERN='(msg|toolu|asst|thread|run|req|session)_[A-Za-z0-9]{20,}|chatcmpl-[A-Za-z0-9]{20,}|"(parentUuid|sessionId|leafUuid|isSidechain)"|claude\.ai/code/[A-Za-z0-9_-]{16,}'
    WHY='a real agent-session artifact (a vendor resource id, a Claude Code transcript key, or an agent-session URL). Session transcripts are conversation content — they do not belong in a published crate, and by operator ruling no session URL belongs in the published text of this repository at all.' ;;

  forbidden-path)
    WHY='a file shape that carries credentials or session state. It should be gitignored, not tracked.' ;;

  binary-content)
    WHY='content the scanner cannot read — a binary the gate would have to take on faith. Track it only if it is a regenerable derivation with a byte-for-byte test, and add it to BINARY_ALLOWED; otherwise do not track it.' ;;
  esac
}

RULES=(private-key vendor-token credential-assignment ipv4-routable
       ipv6-address asn geo-coordinate mac-address home-path personal-email
       quoted-dialogue session-artifact)

# The path rule. Not a content rule: what is wrong with `.env` is that it
# exists at all, whatever is in it.
FORBIDDEN_PATH='(^|/)(\.env(\..+)?|\.netrc|\.npmrc|\.pypirc|credentials\.json|id_(rsa|dsa|ecdsa|ed25519)(\.pub)?)$|(^|/)(\.ssh|\.aws|\.claude|\.gnupg|\.config/brazen)/|\.(pem|key|p12|pfx|jks|keystore|jsonl|kdbx)$'

# The unreadable-content rule (bl-167d). `grep -I` silently SKIPS binary
# files, so before this rule an archive, a database, a PDF, a HAR capture, a
# screenshot or a built executable passed the whole scan by being unreadable —
# the one class of file most likely to carry a dump. Unreadable is now
# rejected, not skipped, and the allowlist is one line of policy: a tracked
# binary must be a DERIVATION the repo can regenerate and check byte for byte.
#
# The allowlist names three files exactly, and they fail that test honestly:
# the throwaway CA, leaf and leaf key of the loopback TLS recorder
# (src/fetch/request/recorder.rs `include_bytes!`s them). They are a test PKI
# minted for 127.0.0.1 and trusted by nothing, random by construction and so
# not regenerable byte for byte. Anything else unreadable is refused.
BINARY_ALLOWED='^src/fetch/firefox_tls/testdata/(ca|leaf|leaf\.key)\.der$'

# Every non-comment line of every content-rule fixture must contain this
# marker (bl-167d). No regex can tell a real secret from a fabricated one, so
# the fixture that houses real-SHAPED values carries its falsity in the value
# itself: a rule fixture cannot be updated by pasting something real without
# the author also typing this word onto the line, and a reviewer reading the
# diff sees it. The scanner is what enforces it; see `--self-test`.
FIXTURE_MARKER='notreal'
