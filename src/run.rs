//! End-to-end glue: CLI → fetch → parse → view → envelope.
//!
//! The stdout/exit-status half of that sentence — one product on stdout and
//! the status that reports what happened to it — lives in [`deliver`].

use crate::cli;
use crate::dom::Document;
use crate::envelope::{kinds, Envelope, ErrorInfo, HttpInfo, NeedsKind, UrlBlock};
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
    // One fetch session per invocation: the document GET, every CSS/JS
    // subfetch, and every redirect ride its shared pool and cache (bl-5191).
    let session = fetch::FetchSession::new(args.headers.clone());
    // One navigation per invocation, and the one way a document GET is issued:
    // every hop of it — an HTTP redirect inside a load, a document-directed
    // re-navigation below — draws on its single hop budget.
    let mut nav = session.navigation(&args.url);
    let mut loaded = nav.load();
    loop {
        let fetched = match loaded {
            Ok(fetched) => fetched,
            Err(e) => {
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
        // A server error (status >= 400) is not an impression: flip to an
        // error envelope before parsing, still carrying the http block.
        if let Some(code) = fetched.status.filter(|&c| c >= 400) {
            let error = ErrorInfo::new(
                format!("http.{code}"),
                format!("server returned HTTP {code}"),
            );
            return Envelope::error(url, args.out, error).with_http(http);
        }
        // A server-declared deferral (needs.md §3: Retry-After on a
        // success response, `cf-mitigated: challenge`) means the real
        // page was never served. Flip to `needs:["human"]` pre-parse and
        // for every view, like the >= 400 flip above — no `out`, no `js`
        // block, and a declared challenge's scripts are never executed.
        if needs::challenge(&fetched.headers) {
            return Envelope::needs(url, args.out, vec![NeedsKind::Human], None).with_http(http);
        }
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
        let doc = Document::parse(&fetched.body);
        // A zero-delay refresh declared by the response *as fetched* — its
        // `Refresh` header or a `<meta http-equiv=refresh>` — is the
        // declarative spelling of a redirect (`src/refresh.rs`): the
        // navigation continues to its target and this document is never the
        // impression — so, like a 3xx body, its scripts are not run.
        if let Some(target) = crate::refresh::target(&doc, &fetched.final_url, &fetched.headers) {
            loaded = nav.follow(&target, &fetched.final_url);
            continue;
        }
        let landed = Landed { url, http, fetched };
        return impression(args, doc, landed, &session, bounds);
    }
}

/// The response's declared media type when it is not a document frot parses
/// (`src/fetch/media.rs`), bounded for quoting. Reads the *same* `content-type`
/// the `http` block surfaces, so the refusal and its evidence agree.
fn declared_non_document(fetched: &FetchResult) -> Option<String> {
    fetch::non_document(fetch::header_value(&fetched.headers, "content-type").as_deref())
}

mod gather;
mod impression;

use impression::{impression, Landed};

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
mod js_concurrency_tests;

#[cfg(test)]
mod golden_tests;

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
