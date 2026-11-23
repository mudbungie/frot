//! The document's target element (HTML, "the indicated part of the
//! document"): the element the URL's fragment names, which `:target` matches
//! and which keeps `autofocus` from taking focus.
//!
//! Measured in Chrome 158 (`bl-5852`): `#n` names the first element whose
//! `id` is `n`, else the first `<a name=n>` — an `id` later in the tree beats
//! an earlier `<a name>`. That is tried with the fragment as written, then
//! percent-decoded (`#%C3%A9` names `id=é`, `#%41` names `id=%41` before
//! `id=A`). An empty fragment, `#top` and a name nothing carries indicate no
//! element.
//!
//! Chrome sets the target once parsing ends, so a script that runs while the
//! page parses sees none; frot's scripts see it from the start.

use super::{Document, Element, NodeId, NodeKind, WalkEvent};
use percent_encoding::percent_decode_str;
use url::Url;

impl Document {
    /// Take the document's URL. Only its fragment is kept: it is the one part
    /// of the address the document's own state depends on.
    pub fn set_url(&mut self, url: &str) {
        let url = Url::parse(url).ok();
        self.fragment = url.and_then(|u| u.fragment().map(str::to_owned));
    }

    /// The target element, if the URL's fragment indicates one.
    pub fn target(&self) -> Option<NodeId> {
        let raw = self.fragment.as_deref()?;
        let decoded = percent_decode_str(raw).decode_utf8_lossy();
        let indicated = [raw, &decoded].into_iter().filter(|f| !f.is_empty());
        let found = indicated.into_iter().find_map(|f| {
            let anchor = || self.first(|el| el.name == "a" && el.attr("name") == Some(f));
            self.element_by_id(f).or_else(anchor)
        });
        found
    }

    /// The first element in tree order that `pick` accepts.
    pub(super) fn first(&self, pick: impl Fn(&Element) -> bool) -> Option<NodeId> {
        let mut found = None;
        self.walk(None, &mut |ev, e| {
            if let (WalkEvent::Enter(id), NodeKind::Element(el)) = (ev, &e.kind) {
                if found.is_none() && pick(el) {
                    found = Some(id);
                }
            }
        });
        found
    }
}

#[cfg(test)]
mod tests;
