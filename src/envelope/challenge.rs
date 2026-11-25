//! The challenge signal (`docs/design/challenge.md` §5.1): what it took to
//! deliver this envelope when the first answer was a stand-in instead of the
//! page — a challenge a response header declared, or an interstitial whose
//! script asked to reload its own document.
//!
//! Present only when either happened; a page that did neither serializes
//! exactly as it always has. `http` reports the **delivered** response; this
//! block reports that a stand-in preceded it, so a clean 200 does not hide it.

use serde::{Deserialize, Serialize};

/// What the round trip cost. `declared` is the response header that declared
/// a challenge — always one of [`super::http::SURFACED`], the same capture
/// `needs::challenge` decides on — and is absent when no header did (the round
/// was a script-asked reload, `bl-14f9`). `rounds` counts navigations frot
/// issued **after** the first because the page's own script changed the
/// request (`0`: frot stopped without one). `passed` is whether the last of
/// those came back as something other than a declared challenge.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChallengeInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared: Option<String>,
    pub rounds: u8,
    pub passed: bool,
}
