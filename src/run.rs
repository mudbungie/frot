//! End-to-end glue: CLI → fetch → parse → view → envelope.
//!
//! The stdout/exit-status half of that sentence — one product on stdout and
//! the status that reports what happened to it — lives in [`deliver`].

use crate::ax;
use crate::cli;
use crate::dom::Document;
use crate::envelope::{
    kinds, Envelope, ErrorInfo, HttpInfo, JsInfo, JsMessage, NeedsKind, UrlBlock, View,
};
use crate::fetch::{self, FetchResult};
use crate::js::{Env, StyleSource};
use crate::needs;
use crate::run::gather::external_css;
use crate::views;
use serde_json::Value;

mod deliver;

pub use deliver::run;
// The `dyn Write` entry point every end-to-end test drives; production reaches
// the pipeline through `run` above.
#[cfg(test)]
pub(crate) use deliver::run_io;

fn build_envelope(args: &cli::Args) -> Envelope {
    let initial_url = UrlBlock::requested(&args.url);
    // One fetch session per invocation: the document GET, every CSS/JS
    // subfetch, and every redirect ride its shared pool and cache (bl-5191).
    let session = fetch::FetchSession::new(args.headers.clone());
    match session.navigate(&args.url) {
        Err(e) => Envelope::error(initial_url, args.out, ErrorInfo::new(&e.kind, e.message)),
        Ok(fetched) => {
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
                return Envelope::needs(url, args.out, vec![NeedsKind::Human], None)
                    .with_http(http);
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
            // §9: JS runs before needs/CSS/layout/views, so everything downstream
            // consumes the post-JS document exactly as it consumes a static one.
            let (doc, js) = run_scripts(args, doc, &fetched, &session);
            // Styles come before needs: a subtree the recipe does not render
            // carries no content, so the starvation detector consults the same
            // cascade the views will (`needs.md` §4 — a CSS-hidden fallback
            // must not mask a dead app as `ok`).
            let styles = compute_styles(args, &doc, &fetched, &session);
            let needs = needs::detect(args.out, &doc, styles.as_ref());
            if !needs.is_empty() {
                return Envelope::needs(url, args.out, needs, None)
                    .with_http(http)
                    .with_js(js);
            }
            // Layout is on-demand (`layout.md` §3): `bboxes` always needs
            // geometry, and `ax` forces layout under `--css` — flex
            // `order`/`*-reverse` is the sole source-order↔reading-order
            // divergence (§3), so without `--css` no reorder is possible and
            // layout is skipped. Both paths have a `Styles` here (`bboxes`
            // always, `ax` because `--css` built one), so layout is built iff
            // the view demands it.
            let wants_layout = args.out == View::Bboxes || (args.out == View::Ax && args.css);
            let layout = wants_layout
                .then(|| styles.as_ref().map(|s| build_layout(s, &doc)))
                .flatten();
            let payload = build_payload(args.out, &doc, &fetched, styles.as_ref(), layout.as_ref());
            Envelope::ok(url, args.out, payload)
                .with_http(http)
                .with_js(js)
        }
    }
}

/// The response's declared media type when it is not a document frot parses
/// (`src/fetch/media.rs`), bounded for quoting. Reads the *same* `content-type`
/// the `http` block surfaces, so the refusal and its evidence agree.
fn declared_non_document(fetched: &FetchResult) -> Option<String> {
    fetch::non_document(fetch::header_value(&fetched.headers, "content-type").as_deref())
}

/// Run page scripts before the rest of the pipeline when `--js` is on (js.md
/// §9), returning the (possibly mutated) document and the envelope `js` block —
/// present only under `--js`. Without it the document passes through untouched.
/// The geometry cache's [`StyleSource`] mirrors the `--css` policy (§8), and the
/// [`Env`] carries the final URL and the User-Agent frot sent (§7). Under
/// `--js-errors` the block also carries the bounded `js.messages` detail (§10).
fn run_scripts(
    args: &cli::Args,
    doc: Document,
    fetched: &FetchResult,
    session: &fetch::FetchSession,
) -> (Document, Option<JsInfo>) {
    if !args.js {
        return (doc, None);
    }
    let styles = if args.css {
        StyleSource::Authored(external_css(&doc, &fetched.final_url, session))
    } else {
        StyleSource::Bare
    };
    let env = Env {
        url: fetched.final_url.clone(),
        user_agent: fetch::user_agent(&args.headers),
        accept_language: fetch::accept_language(&args.headers),
    };
    let (doc, r) = crate::js::run(doc, styles, env, session);
    let mut info = JsInfo::new(r.scripts, r.errors, r.stopped);
    if args.js_errors {
        info = info.with_messages(
            r.messages
                .iter()
                .map(|m| JsMessage::new(&m.kind, &m.text))
                .collect(),
        );
    }
    (doc, Some(info))
}

/// The [`crate::css::Styles`] a view needs. Under `--css`, the full author
/// cascade (external `<link>` sheets + `<style>` + inline + UA) for every view
/// that consults styles. Without `--css`, only `bboxes` needs one — layout
/// needs `display` — sourced *bare* ([`crate::css::compute_bare`]): UA-implicit
/// display + inline `style=` only, no `<style>`/external, so `--css` keeps its
/// "apply author CSS" meaning everywhere (`layout.md` §3). Every other view
/// keeps `None` without `--css`.
fn compute_styles(
    args: &cli::Args,
    doc: &Document,
    fetched: &FetchResult,
    session: &fetch::FetchSession,
) -> Option<crate::css::Styles> {
    if args.css {
        let external = external_css(doc, &fetched.final_url, session);
        Some(crate::css::compute_with(doc, &external, args.js))
    } else if args.out == View::Bboxes {
        Some(crate::css::compute_bare(doc, args.js))
    } else {
        None
    }
}

/// Compute the layout table for a view that demands geometry, at the fixed
/// 1280px viewport (`layout.md` §4).
fn build_layout(styles: &crate::css::Styles, doc: &Document) -> crate::layout::Layout {
    crate::layout::compute(doc, styles, crate::layout::VIEWPORT_WIDTH)
}

fn build_payload(
    view: View,
    doc: &Document,
    fetched: &FetchResult,
    styles: Option<&crate::css::Styles>,
    layout: Option<&crate::layout::Layout>,
) -> Value {
    let page_url = fetched.final_url.as_str();
    match view {
        View::Dom => views::dom::dom_json(doc),
        View::Text => Value::String(views::text::text(doc, styles)),
        View::Links => views::links::links(doc, page_url),
        View::Forms => views::forms::forms(doc, page_url),
        View::Meta => views::meta::meta(doc, page_url),
        View::Ax => ax::ax_tree(doc, styles, layout),
        // `bboxes` always builds both (`compute_styles`/`build_layout` above).
        View::Bboxes => {
            let (layout, styles) = layout
                .zip(styles)
                .expect("bboxes always builds a layout and styles");
            views::bboxes::bboxes(doc, layout, styles)
        }
    }
}

mod gather;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod file_tests;

#[cfg(test)]
mod header_tests;

#[cfg(test)]
mod cookie_tests;

#[cfg(test)]
mod http_tests;

#[cfg(test)]
mod media_tests;

#[cfg(test)]
mod challenge_tests;

#[cfg(test)]
mod bboxes_tests;

#[cfg(test)]
mod ax_tests;

#[cfg(test)]
mod js_tests;

#[cfg(test)]
mod js_concurrency_tests;

#[cfg(test)]
mod golden_tests;

#[cfg(test)]
mod golden_shim_tests;

#[cfg(test)]
mod golden_field_tests;

#[cfg(test)]
mod persona_tests;
