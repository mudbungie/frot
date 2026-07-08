//! End-to-end glue: CLI → fetch → parse → view → envelope → stdout.
//!
//! Exit codes:
//! - `0` — help/version, or `ok`/`needs` envelope emitted.
//! - `1` — `error` envelope emitted (fetch failure or unsupported view).
//! - `2` — usage error (no envelope emitted; message on stderr).

use std::io::Write;

use crate::ax;
use crate::cli;
use crate::dom::{Document, NodeKind, WalkEvent};
use crate::envelope::{Envelope, ErrorInfo, HttpInfo, StatusKind, UrlBlock, View};
use crate::fetch::{self, FetchResult};
use crate::needs;
use crate::views;
use serde_json::Value;
use url::Url;

pub fn run(argv: &[String]) -> u8 {
    run_io(argv, &mut std::io::stdout(), &mut std::io::stderr())
}

pub(crate) fn run_io(argv: &[String], out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let args = match cli::parse(argv) {
        Ok(a) => a,
        Err(e) if e.is_help_or_version() => {
            let _ = writeln!(out, "{}", e);
            return 0;
        }
        Err(e) => {
            let _ = writeln!(err, "{}", e);
            return 2;
        }
    };

    let env = build_envelope(&args);
    let exit = match env.status {
        StatusKind::Ok | StatusKind::Needs => 0,
        StatusKind::Error => 1,
    };
    let _ = writeln!(out, "{}", env.to_json_string());
    exit
}

fn build_envelope(args: &cli::Args) -> Envelope {
    let initial_url = UrlBlock::requested(&args.url);
    match fetch::fetch(&args.url, &args.headers) {
        Err(e) => {
            Envelope::error(initial_url, args.out, ErrorInfo::new(&e.kind, e.message))
        }
        Ok(fetched) => {
            let http = fetched.status.map(HttpInfo::new);
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
            let doc = Document::parse(&fetched.body);
            let needs = needs::detect(args.out, &doc);
            if !needs.is_empty() {
                return Envelope::needs(url, args.out, needs, None).with_http(http);
            }
            let styles = compute_styles(args, &doc, &fetched);
            // Layout is on-demand (`layout.md` §3): only `bboxes` needs geometry,
            // and it always has a `Styles` (built above), so layout is built iff
            // the view is `bboxes`.
            let layout = matches!(args.out, View::Bboxes)
                .then(|| styles.as_ref().map(|s| build_layout(s, &doc)))
                .flatten();
            let payload =
                build_payload(args.out, &doc, &fetched, styles.as_ref(), layout.as_ref());
            Envelope::ok(url, args.out, payload).with_http(http)
        }
    }
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
) -> Option<crate::css::Styles> {
    if args.css {
        let external = external_css(doc, &fetched.final_url, &args.headers);
        Some(crate::css::compute_with(doc, &external))
    } else if args.out == View::Bboxes {
        Some(crate::css::compute_bare(doc))
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
        View::Ax => ax::ax_tree(doc, styles),
        // `bboxes` always builds both (`compute_styles`/`build_layout` above).
        View::Bboxes => {
            let (layout, styles) = layout
                .zip(styles)
                .expect("bboxes always builds a layout and styles");
            views::bboxes::bboxes(doc, layout, styles)
        }
    }
}

/// Absolute URLs of `<link rel="stylesheet">` hrefs, resolved against the
/// page's final URL. Non-stylesheet links, empty hrefs, and hrefs that fail
/// to resolve are dropped. A `file:` sheet is kept only when the page itself
/// is `file:` — remote content must never cause local reads.
fn external_hrefs(doc: &Document, base: &str) -> Vec<String> {
    let base_url = Url::parse(base).ok();
    let base_is_file = base_url.as_ref().is_some_and(|b| b.scheme() == "file");
    let mut out = Vec::new();
    doc.walk(None, &mut |ev, e| {
        if let WalkEvent::Enter(_) = ev {
            if let NodeKind::Element(el) = &e.kind {
                let is_sheet = el.name == "link"
                    && el.attr("rel").is_some_and(|r| {
                        r.split_whitespace().any(|t| t.eq_ignore_ascii_case("stylesheet"))
                    });
                if is_sheet {
                    if let Some(h) = el.attr("href").filter(|s| !s.is_empty()) {
                        if let Some(u) = base_url.as_ref().and_then(|b| b.join(h).ok()) {
                            if u.scheme() != "file" || base_is_file {
                                out.push(u.to_string());
                            }
                        }
                    }
                }
            }
        }
    });
    out
}

/// Fetch each external stylesheet best-effort: CSS is non-critical, so a
/// failed fetch is silently skipped rather than failing the run. The
/// caller's `-H` headers ride along only when the sheet shares the page's
/// origin — credentials never leak cross-origin.
fn external_css(doc: &Document, base: &str, headers: &[(String, String)]) -> Vec<String> {
    external_hrefs(doc, base)
        .into_iter()
        .filter_map(|u| {
            let h = if fetch::same_origin(&u, base) { headers } else { &[] };
            fetch::fetch(&u, h).ok().map(|r| r.body)
        })
        .collect()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod file_tests;

#[cfg(test)]
mod header_tests;

#[cfg(test)]
mod http_tests;

#[cfg(test)]
mod bboxes_tests;
