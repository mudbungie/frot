//! Bounded JS error-message capture (js.md §10). The counted `js.errors` tally
//! answers *how many* failures a run had; this sink answers *what they said*,
//! surfaced in the envelope only behind `--js-errors`. Capture is unconditional
//! but bounded to [`MESSAGES_MAX`], so a noisy page cannot grow the host's
//! memory: the count keeps climbing, the detail array does not.

use std::cell::RefCell;
use std::rc::Rc;

/// The first [`MESSAGES_MAX`] captured error messages. A page that reports more
/// keeps counting into `js.errors`; only the detail is bounded.
pub const MESSAGES_MAX: usize = 32;

/// Characters kept from one message's text. The count bounds *how many*
/// diagnostics a run emits; this bounds how big each may be, so a message whose
/// text is a payload — a `data:` script source, a thrown megastring — names the
/// failure without copying the page into the report. Together they cap the
/// `--js-errors` detail at a few KiB whatever the page does.
pub const TEXT_MAX: usize = 200;

/// One captured JS error (its class + text), in occurrence order. `kind` labels
/// the class — `throw` (script/module exception), `report` (reportError /
/// `window.onerror` / a dispatched window `'error'`), or `subfetch` (a failed
/// external `<script src>`). Rejections, refused navigations, and timer /
/// lifecycle-listener throws stay count-only (js.md §10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub kind: String,
    pub text: String,
}

/// The shared capture sink, written by the host (`js::run`) and the `report`
/// syscall alike, read once after the run.
pub type Messages = Rc<RefCell<Vec<Message>>>;

/// Append one captured message, honouring the [`MESSAGES_MAX`] bound — the
/// single write path for every capture class.
pub fn push(sink: &Messages, kind: &str, text: &str) {
    let mut v = sink.borrow_mut();
    if v.len() < MESSAGES_MAX {
        v.push(Message {
            kind: kind.to_string(),
            text: clamp(text),
        });
    }
}

/// One message's text, clamped to [`TEXT_MAX`] characters with an ellipsis
/// marking the cut — on a character boundary, so the result is still text.
fn clamp(text: &str) -> String {
    match text.char_indices().nth(TEXT_MAX) {
        Some((i, _)) => format!("{}\u{2026}", &text[..i]),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_stops_at_the_bound_but_the_first_n_survive() {
        let sink: Messages = Rc::new(RefCell::new(Vec::new()));
        for i in 0..(MESSAGES_MAX + 5) {
            push(&sink, "throw", &format!("e{i}"));
        }
        let v = sink.borrow();
        assert_eq!(v.len(), MESSAGES_MAX);
        assert_eq!(
            v[0],
            Message {
                kind: "throw".into(),
                text: "e0".into()
            }
        );
        // The last accepted message is the MESSAGES_MAX-th (index MAX-1); the
        // overflow past the bound is dropped, not rotated.
        assert_eq!(v[MESSAGES_MAX - 1].text, format!("e{}", MESSAGES_MAX - 1));
    }

    #[test]
    fn a_long_message_is_clamped_on_a_character_boundary() {
        let sink: Messages = Rc::new(RefCell::new(Vec::new()));
        // Multi-byte characters: a byte-wise cut would split one and panic.
        push(&sink, "subfetch", &"é".repeat(TEXT_MAX * 3));
        let v = sink.borrow();
        assert_eq!(v[0].text.chars().count(), TEXT_MAX + 1);
        assert!(v[0].text.ends_with('\u{2026}'));
        assert!(v[0].text.starts_with("éé"));
    }
}
