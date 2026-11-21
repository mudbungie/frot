//! The landed document → its envelope: scripts, styles, `needs` detection,
//! layout, and the requested view. `run.rs` owns everything before the parse —
//! the navigation and its refusals; this is everything after.

use crate::ax;
use crate::cli;
use crate::dom::Document;
use crate::envelope::{Envelope, HttpInfo, JsInfo, JsMessage, UrlBlock, View};
use crate::fetch::{self, FetchResult};
use crate::js::{Bounds, Env, StyleSource};
use crate::needs;
use crate::run::gather::external_css;
use crate::views;
use serde_json::Value;

/// Where the navigation came to rest: the response and its two envelope blocks.
pub(super) struct Landed {
    pub url: UrlBlock,
    pub http: Option<HttpInfo>,
    pub fetched: FetchResult,
}

/// The envelope for a landed, parsed document: scripts, styles, `needs`
/// detection, then the requested view.
pub(super) fn impression(
    args: &cli::Args,
    doc: Document,
    landed: Landed,
    session: &fetch::FetchSession,
    bounds: Bounds,
) -> Envelope {
    let Landed { url, http, fetched } = landed;
    let fetched = &fetched;
    // §9: JS runs before needs/CSS/layout/views, so everything downstream
    // consumes the post-JS document exactly as it consumes a static one.
    let (doc, js) = run_scripts(args, doc, fetched, session, bounds);
    // Styles come before needs: a subtree the recipe does not render
    // carries no content, so the starvation detector consults the same
    // cascade the views will (`needs.md` §4 — a CSS-hidden fallback
    // must not mask a dead app as `ok`).
    let styles = compute_styles(args, &doc, fetched, session);
    let needs = needs::detect(args.out, &doc, styles.as_ref());
    if !needs.is_empty() {
        return Envelope::needs(url, args.out, needs, None)
            .with_http(http)
            .with_js(js);
    }
    let payload = build_payload(args.out, &doc, fetched, styles.as_ref());
    Envelope::ok(url, args.out, payload)
        .with_http(http)
        .with_js(js)
}

/// Run page scripts before the rest of the pipeline when `--js` is on (js.md
/// §9), returning the (possibly mutated) document and the envelope `js` block —
/// present only under `--js`. Without it the document passes through untouched.
/// The geometry cache's [`StyleSource`] mirrors the `--css` policy (§8), and the
/// [`Env`] carries the final URL and the User-Agent frot sent (§7). Under
/// `--js-errors` the block also carries the bounded `js.messages` detail (§10).
/// `bounds` are the run's two §5 budgets, taken through the call signature so
/// the golden suite can meter its mock-served fixtures on a clock it owns.
pub(super) fn run_scripts(
    args: &cli::Args,
    doc: Document,
    fetched: &FetchResult,
    session: &fetch::FetchSession,
    bounds: Bounds,
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
    let (doc, r) = crate::js::run(doc, styles, env, session, bounds);
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

fn build_payload(
    view: View,
    doc: &Document,
    fetched: &FetchResult,
    styles: Option<&crate::css::Styles>,
) -> Value {
    let page_url = fetched.final_url.as_str();
    match view {
        View::Dom => views::dom::dom_json(doc),
        View::Text => Value::String(views::text::text(doc, styles)),
        View::Links => views::links::links(doc, page_url),
        View::Forms => views::forms::forms(doc, page_url),
        View::Meta => views::meta::meta(doc, page_url),
        View::Ax => ax::ax_tree(doc, styles),
        // Layout is on-demand (`layout.md` §3) and `bboxes` is the one view
        // whose output is geometry, so it is built here, at the fixed 1280px
        // viewport (§4). `compute_styles` always gives `bboxes` a `Styles`.
        View::Bboxes => {
            let styles = styles.expect("bboxes always builds styles");
            let layout = crate::layout::compute(doc, styles, crate::layout::VIEWPORT_WIDTH);
            views::bboxes::bboxes(doc, &layout, styles)
        }
    }
}
