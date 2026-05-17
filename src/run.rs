//! End-to-end glue: CLI → fetch → parse → view → envelope → stdout.
//!
//! Exit codes:
//! - `0` — help/version, or `ok`/`needs` envelope emitted.
//! - `1` — `error` envelope emitted (fetch failure or unsupported view).
//! - `2` — usage error (no envelope emitted; message on stderr).

use std::io::Write;

use crate::ax;
use crate::cli;
use crate::dom::Document;
use crate::envelope::{kinds, Envelope, ErrorInfo, StatusKind, UrlBlock, View};
use crate::fetch::{self, FetchResult};
use crate::needs;
use crate::views;
use serde_json::Value;

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
    match fetch::fetch(&args.url) {
        Err(e) => {
            Envelope::error(initial_url, args.out, ErrorInfo::new(&e.kind, e.message))
        }
        Ok(fetched) => {
            let url = UrlBlock::resolved(&args.url, &fetched.final_url);
            let doc = Document::parse(&fetched.body);
            let needs = needs::detect(args.out, &doc);
            if !needs.is_empty() {
                return Envelope::needs(url, args.out, needs, None);
            }
            let styles = args.css.then(|| crate::css::compute(&doc));
            match build_payload(args.out, &doc, &fetched, styles.as_ref()) {
                Ok(payload) => Envelope::ok(url, args.out, payload),
                Err(e) => Envelope::error(url, args.out, e),
            }
        }
    }
}

fn build_payload(
    view: View,
    doc: &Document,
    fetched: &FetchResult,
    styles: Option<&crate::css::Styles>,
) -> Result<Value, ErrorInfo> {
    let page_url = fetched.final_url.as_str();
    match view {
        View::Dom => Ok(views::dom::dom_json(doc)),
        View::Text => Ok(Value::String(views::text::text(doc, styles))),
        View::Links => Ok(views::links::links(doc, page_url)),
        View::Forms => Ok(views::forms::forms(doc, page_url)),
        View::Meta => Ok(views::meta::meta(doc, page_url)),
        View::Ax => Ok(ax::ax_tree(doc, styles)),
        View::Bboxes => Err(ErrorInfo::new(
            kinds::INTERNAL,
            "--out bboxes lands with the Phase-3 layout capability".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests;
