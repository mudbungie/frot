//! The `ax` projection (`fidelity.md` §3.1): Chrome's full accessibility tree
//! pruned onto frot's declared model — "roles, accessible names, levels" —
//! and both sides rendered as one outline: a line per node, its children
//! between a `(` line and a `)` line. Nesting is spelled by those brackets
//! rather than by indentation so that a delta stays local — one extra wrapper
//! is three lines, not a re-indentation of everything beneath it.
//!
//! The direction is fixed: the browser's channel is mapped onto frot's model,
//! never frot toward the browser. frot has no text nodes, no line boxes, no
//! list markers and none of the widgetry Chrome builds in user-agent shadow
//! trees, so those leave Chrome's side here and are compared nowhere.

use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// What becomes of a Chrome node in frot's model.
enum Fate {
    /// A node frot also has, under this role name.
    Keep(&'static str),
    /// No node of its own; its children take its place.
    Splice,
    /// Gone with everything under it.
    Drop,
}

/// Blink's role vocabulary, mapped onto frot's. One finite table; a role not
/// in it passes through under Blink's own name, so an unmapped role shows up
/// as a delta to disposition rather than vanishing.
fn fate(role: &str) -> Option<Fate> {
    Some(match role {
        // Text granularity and layout objects: outside frot's model.
        "StaticText" | "InlineTextBox" | "ListMarker" | "LineBreak" => Fate::Drop,
        // Wrappers frot never emits a node for (`src/ax/tree.rs`).
        "generic" | "none" | "presentation" => Fate::Splice,
        // Blink-internal roles for elements HTML-AAM gives no role, which
        // frot's vocabulary — ARIA roles — therefore has no word for.
        "LabelText" | "Legend" | "Canvas" | "Video" | "Audio" | "MenuListPopup" | "Abbr" => {
            Fate::Splice
        }
        // Chrome's verdict that a table is layout, which frot renders as
        // demotion to presentation: the scaffolding collapses.
        "LayoutTable" | "LayoutTableRow" | "LayoutTableCell" => Fate::Splice,
        // Blink's names for roles ARIA spells otherwise.
        "image" => Fate::Keep("img"),
        "DisclosureTriangle" => Fate::Keep("button"),
        "SvgRoot" => Fate::Keep("graphics-document"),
        "Figcaption" => Fate::Keep("caption"),
        "DescriptionList" => Fate::Keep("list"),
        _ => return None,
    })
}

struct Tree<'a> {
    nodes: HashMap<&'a str, &'a Value>,
    ua_shadow: HashSet<u64>,
}

impl Tree<'_> {
    /// The projected outline of `node`'s children.
    fn below(&self, node: &Value) -> Vec<String> {
        let mut out = Vec::new();
        for id in node["childIds"].as_array().into_iter().flatten() {
            if let Some(child) = id.as_str().and_then(|id| self.nodes.get(id)) {
                self.walk(child, &mut out);
            }
        }
        out
    }

    fn walk(&self, node: &Value, out: &mut Vec<String>) {
        let role = node["role"]["value"].as_str().unwrap_or("");
        // An ignored node is one Chrome itself leaves out of the tree, and a
        // node inside a user-agent shadow root is in no page's markup; either
        // way its page-authored descendants are still the page's.
        let absent = node["ignored"] == true
            || node["backendDOMNodeId"]
                .as_u64()
                .is_some_and(|id| self.ua_shadow.contains(&id));
        let role = match (absent, fate(role)) {
            (_, Some(Fate::Drop)) => return,
            (true, _) | (_, Some(Fate::Splice)) => return out.extend(self.below(node)),
            (false, Some(Fate::Keep(mapped))) => mapped,
            (false, None) => role,
        };
        let level = node["properties"].as_array().and_then(|props| {
            let level = props.iter().find(|p| p["name"] == "level")?;
            level["value"]["value"].as_u64()
        });
        // frot's levels are heading levels; Chrome also numbers list nesting
        // under the same property name.
        let level = level.filter(|_| role == "heading");
        node_lines(
            role,
            node["name"]["value"].as_str(),
            level,
            self.below(node),
            out,
        );
    }
}

fn node_lines(
    role: &str,
    name: Option<&str>,
    level: Option<u64>,
    below: Vec<String>,
    out: &mut Vec<String>,
) {
    let level = level.map(|l| format!(" level={l}")).unwrap_or_default();
    out.push(format!("{role} {:?}{level}", name.unwrap_or("").trim()));
    if !below.is_empty() {
        out.push("(".to_string());
        out.extend(below);
        out.push(")".to_string());
    }
}

/// Chrome's side: the capture's raw `Accessibility.getFullAXTree` nodes, from
/// the page root down. The root itself is the document, which frot's view
/// starts beneath.
pub fn chrome(capture: &Value) -> Vec<String> {
    let all = capture["ax"].as_array().expect("capture.ax");
    let tree = Tree {
        nodes: all
            .iter()
            .map(|n| (n["nodeId"].as_str().expect("nodeId"), n))
            .collect(),
        ua_shadow: capture["ua_shadow"]
            .as_array()
            .expect("capture.ua_shadow")
            .iter()
            .filter_map(Value::as_u64)
            .collect(),
    };
    let root = all
        .iter()
        .find(|n| n.get("parentId").is_none())
        .expect("an AX root");
    assert_eq!(
        root["role"]["value"], "RootWebArea",
        "the AX root is the document"
    );
    tree.below(root)
}

/// frot's side: the `ax` view, as the same outline.
pub fn frot(out: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    for n in out.as_array().expect("ax children") {
        let role = n["role"].as_str().expect("ax role");
        node_lines(
            role,
            n["name"].as_str(),
            n["level"].as_u64(),
            frot(&n["children"]),
            &mut lines,
        );
    }
    lines
}
