//! The one non-GET request frot can issue: a page script's own POST
//! (`docs/design/challenge.md` §2/§3).
//!
//! [`ScriptBody`] is the whole enforcement. The transport sends a POST only when
//! handed one, and its constructor is private to `crate::js::subfetch` — so no
//! code outside this module tree can originate a non-GET request: a "submit this
//! form" helper elsewhere in frot would not compile. The body arrives from the
//! prelude (`fetch`/`XMLHttpRequest`), where the script chose the verb, the
//! target and the bytes; frot chooses none of them.

use super::{Outcome, Subfetch};

/// The largest request body a page script may send, per request. A token
/// payload is single-digit KiB; this is an order of magnitude more and far below
/// anything that is a useful upload — frot cannot post a file. A constant, not a
/// flag.
pub const REQ_BODY_BYTES: usize = 64 * 1024;

/// A request body a page script chose to send, with the `Content-Type` it goes
/// out under. Holding one is what makes a request a POST.
pub(crate) struct ScriptBody {
    bytes: Vec<u8>,
    content_type: String,
}

impl ScriptBody {
    /// The only constructor, visible to `crate::js::subfetch` alone. A body over
    /// [`REQ_BODY_BYTES`] does not become a request.
    pub(super) fn from_script(text: &str, content_type: &str) -> Result<Self, String> {
        if text.len() > REQ_BODY_BYTES {
            return Err(format!(
                "request body is {} bytes; limit is {REQ_BODY_BYTES}",
                text.len()
            ));
        }
        Ok(ScriptBody {
            bytes: text.as_bytes().to_vec(),
            content_type: content_type.to_string(),
        })
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn content_type(&self) -> &str {
        &self.content_type
    }
}

impl Subfetch {
    /// POST `text` to `spec` (resolved against the page URL) as the page script
    /// asked. It rides the same seam a GET does — the §5 network budget, the
    /// pooled response-byte budget, the resource timing — and differs in two
    /// ways: the response is **never** frozen into the cache (a POST is not
    /// idempotent), and only an `http(s)` target can receive one.
    pub fn post(&mut self, spec: &str, text: &str, content_type: &str) -> Outcome {
        let url = match self.resolve(spec) {
            Ok(u) if u.starts_with("http://") || u.starts_with("https://") => u,
            Ok(u) => return Outcome::Failed(format!("refused non-http POST: {u}")),
            Err(reason) => return Outcome::Failed(reason),
        };
        let body = match ScriptBody::from_script(text, content_type) {
            Ok(b) => b,
            Err(reason) => return Outcome::Failed(reason),
        };
        self.wire(&url, |session, base, timeout| {
            session.submit(&url, base, &body, timeout)
        })
    }
}

#[cfg(test)]
mod tests;
