//! End-to-end glue: CLI → fetch → parse → view → envelope.
//!
//! The stdout/exit-status half of that sentence — one product on stdout and
//! the status that reports what happened to it — lives in [`deliver`].

use crate::cli;
use crate::dom::Document;
use crate::envelope::{kinds, ChallengeInfo, Envelope, ErrorInfo, HttpInfo, NeedsKind, UrlBlock};
use crate::fetch::{self, FetchResult};
use crate::js::Bounds;
use crate::needs;
#[cfg(test)]
use serde_json::Value;

mod deliver;

pub use deliver::run;
// The `dyn Write` entry point every end-to-end test drives; production reaches
// the pipeline through `run` above.
#[cfg(test)]
pub(crate) use deliver::{run_bounded, run_io};

fn build_envelope(args: &cli::Args, bounds: Bounds) -> Envelope {
    // What a declared challenge or a reload round took
    // (`docs/design/challenge.md` §5.1), reported on whatever envelope the
    // navigation ends in. `None` — neither happened — adds nothing: the
    // ordinary path is byte-identical.
    let mut challenge = None;
    navigate(args, bounds, &mut challenge).with_challenge(challenge)
}

fn navigate(args: &cli::Args, bounds: Bounds, challenge: &mut Option<ChallengeInfo>) -> Envelope {
    // One fetch session per invocation: the document GET, every CSS/JS
    // subfetch, and every redirect ride its shared pool and cache (bl-5191).
    let session = fetch::FetchSession::new(args.headers.clone());
    // One navigation per invocation, and the one way a document GET is issued:
    // every hop of it — an HTTP redirect inside a load, a document-directed
    // re-navigation below — draws on its single hop budget.
    let mut nav = session.navigation(&args.url);
    let mut loaded = nav.load();
    // The `js` block of the challenge document's run, kept for the verdict when
    // the round it earned still ends at a challenge.
    let mut answered = None;
    // Navigations issued after the first (§4.3): one count for the whole
    // invocation, whatever asked for them.
    let mut rounds = 0;
    loop {
        let fetched = match loaded {
            Ok(fetched) => fetched,
            Err(e) => {
                // Nothing came back, so nothing was passed.
                challenge.iter_mut().for_each(|c| c.passed = false);
                let url = UrlBlock::requested(&args.url);
                return Envelope::error(url, args.out, ErrorInfo::new(&e.kind, e.message));
            }
        };
        // One capture, two consumers (needs.md §3): `http` surfaces the
        // allowlisted slice of `fetched.headers` and `needs::challenge`
        // decides from the same set — they cannot disagree.
        let http = fetched
            .status
            .map(|code| HttpInfo::new(code, &fetched.headers));
        let url = UrlBlock::resolved(&args.url, &fetched.final_url);
        // A server-declared deferral (needs.md §3: Retry-After on a success
        // response, `cf-mitigated: challenge`, `x-amzn-waf-action: challenge`)
        // means the real page was never served — at whatever status the
        // vendor chose to say so (Cloudflare's is 403, `bl-a7c5`).
        let declared = needs::challenge(fetched.status, &fetched.headers);
        // After a round, any response that is not a declared challenge is the
        // challenge passed — whatever that response then turns out to be.
        challenge
            .iter_mut()
            .for_each(|c| c.passed = declared.is_none());
        // A server error (status >= 400) that declares no challenge is not an
        // impression: flip to an error envelope before parsing, still
        // carrying the http block.
        let refused = fetched.status.filter(|&c| c >= 400 && declared.is_none());
        if let Some(code) = refused {
            let error = ErrorInfo::new(
                format!("http.{code}"),
                format!("server returned HTTP {code}"),
            );
            return Envelope::error(url, args.out, error).with_http(http);
        }
        // Whether the server asked for the page to be run and the caller's
        // URL asked again: a declared challenge (`docs/design/challenge.md`
        // §4.2).
        let asked = if let Some(header) = declared {
            // The first header to declare one names it; a block a reload
            // round opened keeps that round's count.
            let info = challenge.get_or_insert_with(ChallengeInfo::default);
            info.declared.get_or_insert_with(|| header.to_string());
            let verdict = |kind| Envelope::needs(url.clone(), args.out, vec![kind], None);
            // No `--js`: a recipe change genuinely helps, so say which.
            if !args.js {
                return verdict(NeedsKind::Js).with_http(http);
            }
            // The round is spent and the answer is still a challenge: frot has
            // tried the automatable route, so the escalation is earned.
            if rounds == CHALLENGE_ROUNDS {
                return verdict(NeedsKind::Human).with_http(http).with_js(answered);
            }
            true
        } else {
            // A body the one parser cannot read is not an impression either
            // (`src/fetch/media.rs`): handing html5ever a PNG fabricates
            // elements out of chunk headers, so a declared non-text media type
            // is refused here — pre-parse, every view, `http` block kept.
            if let Some(media) = declared_non_document(&fetched) {
                let error = ErrorInfo::new(
                    kinds::PARSE,
                    format!("response media type {media} is not a document"),
                );
                return Envelope::error(url, args.out, error).with_http(http);
            }
            false
        };
        let doc = Document::parse(&fetched.body);
        // A zero-delay refresh declared by the response *as fetched* — its
        // `Refresh` header or a `<meta http-equiv=refresh>` — is the
        // declarative spelling of a redirect (`src/refresh.rs`): the
        // navigation continues to its target and this document is never the
        // impression — so, like a 3xx body, its scripts are not run. A
        // challenge is answered, not redirected.
        let refresh = crate::refresh::target(&doc, &fetched.final_url, &fetched.headers);
        if let Some(target) = refresh.filter(|_| !asked) {
            loaded = nav.follow(&target, &fetched.final_url);
            continue;
        }
        // frot does what a browser does unattended: it runs the page's own
        // scripts, and re-issues the caller's URL — once — iff it was asked
        // to (the server declared a challenge, or the script asked to reload
        // its own document) and the script changed the request frot would
        // send (§12). It originates nothing and never navigates elsewhere.
        let before = nav.request();
        let (ran, reload) = run_scripts(args, doc, &fetched, &session, bounds.clone());
        let open = (asked || reload) && rounds < CHALLENGE_ROUNDS;
        if open && nav.request().earned_since(&before) {
            rounds += 1;
            // A round opens the block whatever asked for it (§5.1): no header
            // may have declared anything, and the caller still learns that the
            // delivered page is not what the URL first answered.
            challenge.get_or_insert_with(ChallengeInfo::default).rounds = rounds;
            answered = ran.1;
            loaded = nav.load();
            continue;
        }
        // A challenge whose scripts did not change the request — failed, ran
        // out of budget, minted nothing, or only loaded a resource whose
        // response set a cookie — is an honest stop: re-sending it would be a
        // retry loop.
        if asked {
            return Envelope::needs(url, args.out, vec![NeedsKind::Human], None)
                .with_http(http)
                .with_js(ran.1);
        }
        let landed = Landed { url, http, fetched };
        return impression(args, ran, landed, &session);
    }
}

/// Navigations frot will issue **after** the first to answer a declared
/// challenge or a script's request to reload (`docs/design/challenge.md` §4.3,
/// §12): at most two per invocation, ever. A constant, not a flag.
const CHALLENGE_ROUNDS: u8 = 1;

/// The response's declared media type when it is not a document frot parses
/// (`src/fetch/media.rs`), bounded for quoting. Reads the *same* `content-type`
/// the `http` block surfaces, so the refusal and its evidence agree.
fn declared_non_document(fetched: &FetchResult) -> Option<String> {
    fetch::non_document(fetch::header_value(&fetched.headers, "content-type").as_deref())
}

mod gather;
mod impression;

use impression::{impression, run_scripts, Landed};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod view_tests;

#[cfg(test)]
mod file_tests;

#[cfg(test)]
mod header_tests;

#[cfg(test)]
mod cookie_tests;

#[cfg(test)]
mod url_tests;

#[cfg(test)]
mod http_tests;

#[cfg(test)]
mod media_tests;

#[cfg(test)]
mod challenge_tests;

#[cfg(test)]
mod roundtrip_tests;

#[cfg(test)]
mod roundtrip_status_tests;

#[cfg(test)]
mod roundtrip_guard_tests;

#[cfg(test)]
mod roundtrip_subtle_tests;

#[cfg(test)]
mod roundtrip_reload_tests;

#[cfg(test)]
mod bboxes_tests;

#[cfg(test)]
mod css_media_tests;

#[cfg(test)]
mod details_tests;

#[cfg(test)]
mod fallback_tests;

#[cfg(test)]
mod canvas_tests;

#[cfg(test)]
mod ax_tests;

#[cfg(test)]
mod js_tests;

#[cfg(test)]
mod images_tests;

#[cfg(test)]
mod js_concurrency_tests;

#[cfg(test)]
mod golden_tests;

#[cfg(test)]
mod referrer_tests;
#[cfg(test)]
mod refresh_tests;

#[cfg(test)]
mod golden_shim_tests;

#[cfg(test)]
mod golden_field_tests;

#[cfg(test)]
mod golden_bounds_tests;

#[cfg(test)]
mod persona_tests;
