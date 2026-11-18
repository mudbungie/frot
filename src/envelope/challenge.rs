//! The declared-challenge signal (`docs/design/challenge.md` §5.1): what it took
//! to deliver this envelope when the server answered the navigation with a
//! challenge instead of the page.
//!
//! Present only when a challenge was declared; a page that declares none
//! serializes exactly as it always has. `http` reports the **delivered**
//! response; this block reports that a challenge preceded it, so a clean 200
//! does not hide the stand-in that came first.

use serde::{Deserialize, Serialize};

/// What a declared challenge cost. `declared` is the response header that
/// declared it — always one of [`super::http::SURFACED`], the same capture
/// `needs::challenge` decides on. `rounds` counts navigations frot issued
/// **after** the first because the page's own script changed the request (`0`:
/// frot stopped without one). `passed` is whether the last of those came back
/// as something other than a declared challenge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChallengeInfo {
    pub declared: String,
    pub rounds: u8,
    pub passed: bool,
}

impl ChallengeInfo {
    /// A challenge just declared by `header`: no round taken, nothing passed.
    pub fn declared(header: &str) -> Self {
        Self {
            declared: header.to_string(),
            rounds: 0,
            passed: false,
        }
    }
}
