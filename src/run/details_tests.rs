//! A closed `<details>` end-to-end across every view that claims to report
//! rendered content (`bl-74a6`).
//!
//! Field repro 2026-08-11, `https://design-system.service.gov.uk/components/details/`
//! under `--css --out text`: both closed disclosures emitted their entire
//! concealed body ("Use options to customise the appearance, content and
//! behaviour"), and `--out bboxes` emitted the same paragraphs as zero-sized
//! entries. Chrome's oracle: `details.open === false` on both, only the
//! `<summary>` has client rects, no interaction involved.
//!
//! Every view here drops the content in **both** recipes, and each for its own
//! reason: `bboxes` has a `Styles` table without `--css` (`compute_bare`,
//! `layout.md` §3), while `text` and `ax` descend through
//! `Document::painted_children` / `Document::ax_children` and never see the
//! node (`bl-0aaf`, `bl-d470`). `[hidden]` is the rule that really does need
//! `--css` (`bl-eeb4`), because it is an author-overridable UA *declaration*;
//! this one is structural and no author rule can reveal it.

use super::*;

const DISCLOSURE: &str = "<html><body><details><summary>Options</summary>\
     <p>Use options to customise the appearance.</p>raw tail</details></body></html>";

fn run_capture(args: &[&str]) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap())
}

fn serve_and_run(body: &str, args: &[&str]) -> Value {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body(body)
        .create();
    let url = server.url();
    let mut argv = vec![url.as_str()];
    argv.extend_from_slice(args);
    let (code, out) = run_capture(&argv);
    assert_eq!(code, 0);
    serde_json::from_str(out.trim()).expect("envelope JSON")
}

/// Every `(role, name)` in the `ax` payload, in emission order.
fn ax_pairs(v: &Value) -> Vec<(String, String)> {
    fn walk(v: &Value, out: &mut Vec<(String, String)>) {
        if let Value::Array(arr) = v {
            for item in arr {
                out.push((
                    item["role"].as_str().unwrap_or("").to_string(),
                    item["name"].as_str().unwrap_or("").to_string(),
                ));
                walk(&item["children"], out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&v["out"], &mut out);
    out
}

/// Every emitted `tag` in the `bboxes` payload, in reading order.
fn bbox_tags(v: &Value) -> Vec<String> {
    v["out"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["tag"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn text_stops_at_the_summary_in_either_recipe() {
    // `bl-d470`: this used to hold only under `--css`, because the text view
    // read the *tag* half of the structural rule and left the closed-`<details>`
    // half to the cascade. Chrome's `body.innerText` for this markup is
    // "Options" with no CSS involved — the disclosure body is
    // `content-visibility: hidden` while closed, not an author declaration —
    // so the recipe decides how the fact is computed, never what it covers.
    for args in [vec!["--out", "text"], vec!["--css", "--out", "text"]] {
        let v = serve_and_run(DISCLOSURE, &args);
        assert_eq!(v["status"], "ok", "{args:?}");
        assert_eq!(v["out"], "Options", "{args:?}");
    }
    // Opened, the whole disclosure is content again — in either recipe too.
    let opened = DISCLOSURE.replace("<details>", "<details open>");
    for args in [vec!["--out", "text"], vec!["--css", "--out", "text"]] {
        let v = serve_and_run(&opened, &args);
        assert_eq!(v["out"], "Options\nUse options to customise the appearance.\nraw tail");
    }
}

#[test]
fn no_author_rule_can_reveal_the_disclosure_in_the_raw_recipe_either() {
    // `bl-74a6`'s non-overridability, restated for the recipe that has no
    // cascade to be outranked in. The box the UA skips is the `<details>`'s
    // `::details-content`, not the child's, so `display`/`visibility` on the
    // child has nothing to act on — with `--css` the cascade folds concealment
    // in *after* the author origin, and without it `painted_children` never
    // offers the node at all.
    let page = "<html><head><style>details p{display:block!important;\
         visibility:visible!important;content-visibility:visible!important}</style></head>\
         <body><details><summary>Options</summary>\
         <p style='display:block!important'>body</p></details></body></html>";
    for args in [vec!["--out", "text"], vec!["--css", "--out", "text"]] {
        assert_eq!(serve_and_run(page, &args)["out"], "Options", "{args:?}");
    }
}

#[test]
fn ax_exposes_the_control_and_not_the_disclosure() {
    // `<summary>` is a `button`, `<details>` a `group` (HTML-AAM). The control
    // is in the tree; nothing behind it is.
    let v = serve_and_run(DISCLOSURE, &["--css", "--out", "ax"]);
    assert_eq!(
        ax_pairs(&v),
        vec![
            ("group".to_string(), String::new()),
            ("button".to_string(), "Options".to_string()),
        ]
    );
    // Opened, the disclosure content joins the tree.
    let opened = DISCLOSURE.replace("<details>", "<details open>");
    let v = serve_and_run(&opened, &["--css", "--out", "ax"]);
    assert!(
        ax_pairs(&v).iter().any(|(r, _)| r == "paragraph"),
        "{:?}",
        ax_pairs(&v)
    );
}

#[test]
fn bboxes_omits_the_concealed_boxes_rather_than_zeroing_them() {
    // The defect's second face: a zero-sized entry is not an omission, and
    // README defines `bboxes` as one entry per *rendered* element.
    for args in [vec!["--out", "bboxes"], vec!["--css", "--out", "bboxes"]] {
        let tags = bbox_tags(&serve_and_run(DISCLOSURE, &args));
        assert!(tags.contains(&"summary".to_string()), "{args:?} {tags:?}");
        assert!(!tags.contains(&"p".to_string()), "{args:?} {tags:?}");
    }
    let opened = DISCLOSURE.replace("<details>", "<details open>");
    assert!(bbox_tags(&serve_and_run(&opened, &["--out", "bboxes"])).contains(&"p".to_string()));
}

#[test]
fn a_script_that_opens_the_disclosure_reveals_it() {
    // `--js` mutates the one arena and the cascade runs after it (`run.rs`
    // §9), so the toggle is read off the live attribute with no extra seam.
    let page = "<html><body><details><summary>Options</summary><p>revealed</p></details>\
         <script>document.querySelector('details').setAttribute('open','');</script>\
         </body></html>";
    let v = serve_and_run(page, &["--css", "--out", "text"]);
    assert_eq!(v["out"], "Options");
    let v = serve_and_run(page, &["--css", "--js", "--out", "text"]);
    assert_eq!(v["out"], "Options\nrevealed");
}
