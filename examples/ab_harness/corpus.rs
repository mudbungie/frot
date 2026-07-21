//! Corpus definition and per-invocation probing for the A/B harness.
//!
//! The corpus is verbatim from `docs/design/identity.md` §3.6/§3.7 so every row
//! is traceable, plus a JA4/ALPN oracle row (`tls.peet.ws`) that reports frot's
//! wire persona back in its JSON body — the only way a black-box A/B run reads
//! the protocol frot actually negotiated.
//!
//! **Trap #1 (substring content checks lie).** A prior run scored Wikipedia as a
//! captcha page because inline JS contained `wgConfirmEditCaptcha`. This module
//! NEVER substring-matches block-page copy. The meaningful-content signal is
//! purely structural: the element count and the total text length in the `--out
//! dom` tree, which a block/challenge page scores low on and a real page scores
//! high on, regardless of any string a script embeds.
//!
//! **Trap #2 (shell `$?` capture lies).** A prior run mis-read frot as exiting 0
//! on an error envelope because `$?` had captured a following `echo`. This driver
//! never touches `$?`: it reads `std::process::Output::status` — the OS exit
//! status captured atomically by `wait()`, structurally immune to the shell race.

use std::process::Command;
use std::time::Instant;

use serde_json::Value;

use super::sha256;

/// Corpus classes, verbatim from identity.md §3.6/§3.7 plus the oracle.
#[derive(Clone, Copy, PartialEq)]
pub enum Class {
    /// Must not regress (§3.6): real content on both binaries.
    Control,
    /// Soft-gate cases (§3.6): amazon 202, stackoverflow.
    SoftGate,
    /// Declared-challenge negative controls (§3.7): EXACTLY ONE request each,
    /// never executed or retried.
    Challenge,
    /// JA4/ALPN oracle: reports frot's wire persona in its response body.
    Oracle,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Control => "control",
            Class::SoftGate => "soft-gate",
            Class::Challenge => "declared-challenge",
            Class::Oracle => "wire-oracle",
        }
    }
}

/// One corpus row.
pub struct Target {
    pub class: Class,
    pub url: &'static str,
}

const fn t(class: Class, url: &'static str) -> Target {
    Target { class, url }
}

/// The fixed corpus. Order is stable so the before/after table reproduces.
pub const CORPUS: &[Target] = &[
    t(Class::Control, "https://en.wikipedia.org/wiki/Main_Page"),
    t(Class::Control, "https://lobste.rs/"),
    t(Class::Control, "https://news.ycombinator.com/"),
    t(Class::Control, "https://react.dev/"),
    t(Class::SoftGate, "https://stackoverflow.com/questions"),
    t(Class::SoftGate, "https://www.amazon.com/"),
    t(Class::Challenge, "https://www.reddit.com/"),
    t(Class::Challenge, "https://www.g2.com/"),
    t(Class::Oracle, "https://tls.peet.ws/api/all"),
];

/// One invocation's captured outcome.
pub struct Probe {
    pub exit: Option<i32>,
    pub status: String,
    pub needs: String,
    pub error_kind: String,
    pub http_status: Option<u64>,
    /// Declaration headers (`retry-after`/`cf-mitigated`/`server`) if the
    /// envelope surfaces them (bl-acec's `http.headers`); empty if it does not.
    pub decl: String,
    pub bytes: usize,
    pub sha: String,
    /// Structural meaningful-content: element count in the `--out dom` tree.
    pub elements: usize,
    /// Structural meaningful-content: total text length in the `--out dom` tree.
    pub text_chars: usize,
    /// Whether a `js` block is present, i.e. scripts were executed. A challenge
    /// negative control must show this `false` (never executed).
    pub executed: bool,
    /// Wire persona from the oracle body (empty for non-oracle rows).
    pub peetprint: String,
    pub ja4: String,
    pub protocol: String,
    pub wall_ms: u128,
    /// Whether the OS exit code matches the documented contract for `status`.
    pub exit_contract_ok: bool,
}

/// Run one frot invocation and capture everything observable. `bin` is a built
/// frot binary; `recipe` are the flags after the URL.
pub fn probe(bin: &str, url: &str, recipe: &[&str]) -> Probe {
    let start = Instant::now();
    let out = Command::new(bin).arg(url).args(recipe).output();
    let wall_ms = start.elapsed().as_millis();
    let out = match out {
        Ok(o) => o,
        Err(e) => return spawn_failed(bin, &e.to_string(), wall_ms),
    };
    // Trap #2: read the atomically-captured OS status, never a shell `$?`.
    let exit = out.status.code();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let env: Option<Value> = serde_json::from_str(stdout.trim()).ok();
    from_envelope(exit, env.as_ref(), &stdout, wall_ms)
}

fn spawn_failed(bin: &str, msg: &str, wall_ms: u128) -> Probe {
    let mut p = blank(wall_ms);
    p.status = format!("(spawn-failed: {bin}: {msg})");
    p
}

fn blank(wall_ms: u128) -> Probe {
    Probe {
        exit: None,
        status: String::new(),
        needs: String::new(),
        error_kind: String::new(),
        http_status: None,
        decl: String::new(),
        bytes: 0,
        sha: String::new(),
        elements: 0,
        text_chars: 0,
        executed: false,
        peetprint: String::new(),
        ja4: String::new(),
        protocol: String::new(),
        wall_ms,
        exit_contract_ok: false,
    }
}

fn from_envelope(exit: Option<i32>, env: Option<&Value>, stdout: &str, wall_ms: u128) -> Probe {
    let mut p = blank(wall_ms);
    p.exit = exit;
    let Some(env) = env else {
        // No envelope on stdout: the documented shape of a usage error (exit 2,
        // message on stderr). Anything else is a contract violation.
        p.status = "(no-envelope)".into();
        p.exit_contract_ok = exit == Some(2) && stdout.trim().is_empty();
        return p;
    };
    p.status = str_field(env, "status");
    p.needs = env
        .get("needs")
        .and_then(|n| n.as_array())
        .map(|a| join_strs(a))
        .unwrap_or_default();
    p.error_kind = env
        .get("error")
        .and_then(|e| e.get("kind"))
        .and_then(|k| k.as_str())
        .unwrap_or("")
        .to_string();
    p.http_status = env
        .get("http")
        .and_then(|h| h.get("status"))
        .and_then(|s| s.as_u64());
    p.decl = declaration(env);
    p.executed = env.get("js").is_some();
    if let Some(out) = env.get("out") {
        fill_out(&mut p, out);
    }
    p.exit_contract_ok = exit_matches(&p.status, exit);
    p
}

/// The exit contract (`run.rs`): 0 for ok/needs, 1 for an error envelope, 2 for
/// usage. A parsed envelope is never a usage error, so ok/needs→0, error→1.
fn exit_matches(status: &str, exit: Option<i32>) -> bool {
    match status {
        "ok" | "needs" => exit == Some(0),
        "error" => exit == Some(1),
        _ => false,
    }
}

fn fill_out(p: &mut Probe, out: &Value) {
    let text = match out {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    };
    p.bytes = text.len();
    p.sha = sha256::hex(text.as_bytes());
    count_dom(out, &mut p.elements, &mut p.text_chars);
    read_oracle(p, out);
}

/// Recursively count elements and text length in a `--out dom` tree (trap #1:
/// structural, never a substring match).
fn count_dom(v: &Value, elements: &mut usize, text_chars: &mut usize) {
    match v {
        Value::Array(a) => a.iter().for_each(|c| count_dom(c, elements, text_chars)),
        Value::Object(o) => {
            match o.get("type").and_then(|t| t.as_str()) {
                Some("element") => *elements += 1,
                Some("text") => {
                    *text_chars += o.get("value").and_then(|s| s.as_str()).unwrap_or("").len()
                }
                _ => {}
            }
            if let Some(children) = o.get("children") {
                count_dom(children, elements, text_chars);
            }
        }
        _ => {}
    }
}

/// Parse the JA4/ALPN oracle body (`tls.peet.ws/api/all`): its JSON reports the
/// TLS/HTTP frot actually negotiated, the black-box view of the wire persona.
fn read_oracle(p: &mut Probe, out: &Value) {
    let body = match out {
        Value::String(s) => match serde_json::from_str::<Value>(s) {
            Ok(v) => v,
            Err(_) => return,
        },
        _ => return,
    };
    let tls = body.get("tls");
    p.ja4 = tls
        .and_then(|t| t.get("ja4"))
        .and_then(|j| j.as_str())
        .unwrap_or("")
        .to_string();
    p.protocol = str_field(&body, "http_version");
    p.peetprint = tls
        .and_then(|t| t.get("peetprint"))
        .and_then(|j| j.as_str())
        .unwrap_or("")
        .to_string();
}

fn declaration(env: &Value) -> String {
    let Some(headers) = env.get("http").and_then(|h| h.get("headers")) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for name in ["retry-after", "cf-mitigated", "server"] {
        if let Some(v) = lookup_header(headers, name) {
            parts.push(format!("{name}={v}"));
        }
    }
    parts.join(" ")
}

/// bl-acec surfaces `http.headers` as an array of `{"name","value"}` objects
/// (lower-cased names, wire order). Find one by case-insensitive name.
fn lookup_header(headers: &Value, name: &str) -> Option<String> {
    headers.as_array()?.iter().find_map(|h| {
        let obj = h.as_object()?;
        let k = obj.get("name")?.as_str()?;
        k.eq_ignore_ascii_case(name)
            .then(|| obj.get("value")?.as_str().map(str::to_string))
            .flatten()
    })
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn join_strs(a: &[Value]) -> String {
    a.iter()
        .filter_map(|x| x.as_str())
        .collect::<Vec<_>>()
        .join(",")
}
