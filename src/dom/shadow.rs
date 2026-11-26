//! Shadow roots (js.md §15): the tree an element hosts beside its children.
//! A root is an arena node whose kind records its host and the options it
//! was attached with; the host's `content` names it. Nothing that walks the
//! node tree enters it — the flat tree ([`super::flat`]) is where it renders.
//! Slot assignment is a query over the tree, never stored, so it cannot go
//! stale when a page renames a slot or moves a child.

use markup5ever_rcdom::{Handle, NodeData};

use super::{Document, NodeId, NodeKind};

/// What a shadow root was attached with. All of it is fixed at attach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Shadow {
    pub host: NodeId,
    /// `mode: "closed"`: scripts cannot reach the root from its host.
    pub closed: bool,
    /// Made by the parser from a `<template shadowrootmode>`.
    pub declarative: bool,
    pub clonable: bool,
    pub serializable: bool,
    pub delegates_focus: bool,
    /// `slotAssignment: "manual"`: nothing is assigned by name.
    pub manual: bool,
}

/// The HTML elements that may host a shadow root, besides any valid custom
/// element name (HTML "attach a shadow root", step 3).
const HOSTS: [&str; 18] = [
    "article",
    "aside",
    "blockquote",
    "body",
    "div",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "main",
    "nav",
    "p",
    "section",
    "span",
];

impl Document {
    /// The shadow root `id` hosts, if any.
    pub fn shadow_root(&self, id: NodeId) -> Option<NodeId> {
        self.hosted(id).map(|(root, _)| root)
    }

    /// The shadow root `id` hosts and what it was attached with.
    fn hosted(&self, id: NodeId) -> Option<(NodeId, Shadow)> {
        let root = self.node(id).content?;
        self.shadow(root).map(|s| (root, s))
    }

    /// What the shadow root `root` was attached with; `None` for any other node.
    pub fn shadow(&self, root: NodeId) -> Option<Shadow> {
        match self.node(root).kind {
            NodeKind::Shadow(s) => Some(s),
            _ => None,
        }
    }

    /// Attach a shadow root to `init.host` and return it, or Firefox's reason
    /// for refusing (measured, js.md §15.1). Re-attaching onto a declarative
    /// root of the same mode empties that root and answers it.
    pub fn attach_shadow(&mut self, init: Shadow) -> Result<NodeId, &'static str> {
        let host = init.host;
        if !self.element(host).is_some_and(|el| valid_host(&el.name)) {
            return Err("Unable to attach ShadowDOM");
        }
        if let Some((root, was)) = self.hosted(host) {
            if !was.declarative || was.closed != init.closed {
                return Err("Unable to re-attach to existing ShadowDOM");
            }
            for kid in self.node(root).children.clone() {
                self.detach(kid);
            }
            self.nodes[root as usize].kind = NodeKind::Shadow(Shadow {
                declarative: false,
                ..was
            });
            return Ok(root);
        }
        let root = self.append_root(init);
        Ok(root)
    }

    /// The shadow root the parsed child `k` of `host` declares, and the
    /// contents to fill it with: `k` is a `<template>` whose `shadowrootmode`
    /// is `open` or `closed` (ASCII case-insensitive, as Firefox reads it),
    /// and `host` is an element that may host one and has none yet.
    pub(super) fn declarative(&self, k: &Handle, host: NodeId) -> Option<(Shadow, Handle)> {
        let NodeData::Element {
            attrs,
            template_contents,
            ..
        } = &k.data
        else {
            return None;
        };
        let contents = template_contents.borrow().clone()?;
        let attrs = attrs.borrow();
        let attr = |n: &str| {
            attrs
                .iter()
                .find(|a| &*a.name.local == n)
                .map(|a| a.value.to_ascii_lowercase())
        };
        let mode = attr("shadowrootmode")?;
        let valid = self.element(host).is_some_and(|el| valid_host(&el.name));
        if !(mode == "open" || mode == "closed") || !valid || self.node(host).content.is_some() {
            return None;
        }
        let init = Shadow {
            host,
            closed: mode == "closed",
            declarative: true,
            clonable: attr("shadowrootclonable").is_some(),
            serializable: attr("shadowrootserializable").is_some(),
            delegates_focus: attr("shadowrootdelegatesfocus").is_some(),
            manual: attr("shadowrootslotassignment").as_deref() == Some("manual"),
        };
        Some((init, contents))
    }

    /// A new, empty shadow root on `init.host`, which has none.
    pub(super) fn append_root(&mut self, init: Shadow) -> NodeId {
        let root = self.push(NodeKind::Shadow(init), None);
        self.nodes[init.host as usize].content = Some(root);
        self.generation += 1;
        root
    }

    /// Every shadow root whose host is connected, in arena order.
    pub fn shadow_roots(&self) -> Vec<NodeId> {
        let ids = 0..self.len() as NodeId;
        ids.filter(|&n| self.shadow(n).is_some_and(|s| self.connected(s.host)))
            .collect()
    }

    /// The top of `id`'s node tree: the document, a shadow root, or the top
    /// of a detached subtree.
    pub fn tree_root(&self, id: NodeId) -> NodeId {
        let mut n = id;
        while let Some(p) = self.node(n).parent {
            n = p;
        }
        n
    }

    /// The slot `id` is assigned to: the first `<slot>` in its parent's shadow
    /// tree whose `name` is `id`'s `slot` attribute (`''` for text) — in a
    /// manual-assignment root, the slot whose manually assigned nodes hold it.
    /// Only an element or text child of a host has one.
    pub fn assigned_slot(&self, id: NodeId) -> Option<NodeId> {
        let (root, shadow) = self.hosted(self.node(id).parent?)?;
        let want = match &self.node(id).kind {
            NodeKind::Element(el) => el.attr("slot").unwrap_or(""),
            NodeKind::Text(_) => "",
            _ => return None,
        };
        if shadow.manual {
            let mut slots = self.slots_in(root).into_iter();
            return slots.find(|&s| self.manually_assigned(s).contains(&id));
        }
        self.slots_in(root)
            .into_iter()
            .find(|&s| self.element(s).and_then(|el| el.attr("name")).unwrap_or("") == want)
    }

    /// The nodes assigned to the slot `slot`, in tree order: the children of
    /// its tree's host whose assigned slot it is — in a manual-assignment
    /// root, its manually assigned nodes that are the host's children, in
    /// the order given (Firefox 153). Empty for a slot in no shadow tree.
    pub fn assigned(&self, slot: NodeId) -> Vec<NodeId> {
        let Some(s) = self.shadow(self.tree_root(slot)) else {
            return Vec::new();
        };
        if s.manual {
            let mut nodes = self.manually_assigned(slot).to_vec();
            nodes.retain(|&n| self.node(n).parent == Some(s.host));
            return nodes;
        }
        self.node(s.host)
            .children
            .iter()
            .copied()
            .filter(|&c| self.assigned_slot(c) == Some(slot))
            .collect()
    }

    /// The `<slot>` elements of `root`'s tree, in tree order (not those of a
    /// shadow tree nested inside it).
    fn slots_in(&self, root: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        for &c in &self.node(root).children {
            self.walk(Some(c), &mut |ev, entry| {
                if let (super::WalkEvent::Enter(n), NodeKind::Element(el)) = (ev, &entry.kind) {
                    if el.name == "slot" {
                        out.push(n);
                    }
                }
            });
        }
        out
    }
}

/// Whether an element named `name` may host a shadow root.
pub fn valid_host(name: &str) -> bool {
    HOSTS.contains(&name) || crate::css::custom_name(name)
}

mod scope;

#[cfg(test)]
mod tests;
