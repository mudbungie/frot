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
            text: text.to_string(),
        });
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
}
