//! The h2 **request-side** oracle (identity.md §12) — the first HEADERS frame's
//! stream id, its PRIORITY flag, and its pseudo-header order, plus the composed
//! akamai-h2 fingerprint (`bl-7523`).
//!
//! All three are **declared residuals** (§7 stage C, §11): the `h2` crate opens
//! client streams at 1 where the persona opens at 3, exposes no client PRIORITY
//! API, and hardcodes `m,s,a,p`. §12 says residuals are asserted *as* residuals
//! with a pointer to stage C — before this module they were asserted nowhere,
//! and the only mention of `pseudo_order`/`priority_weight` outside the const
//! was a struct-equality check of the const against its own literal, which is a
//! drift guard on the declaration, not an observation of the wire.
//!
//! Everything below **is** an observation of the wire. Frames are selected by
//! kind rather than index because this origin replies with its own SETTINGS —
//! it has to, or the client never reaches the request — which puts the client's
//! SETTINGS ACK in the stream.

use std::time::Duration;

use super::h2_wire::{
    akamai, frames, persona_akamai, pseudo_order, settings, spawn_origin, test_roots, Frame,
    PRIORITY_FLAG,
};
use super::Transport;
use crate::fetch::profile::FIREFOX_140_ESR;
use crate::fetch::MAX_BODY_BYTES;

/// An empty h2 SETTINGS frame — the server preface the client waits for before
/// opening its first stream.
const SERVER_SETTINGS: &[u8] = &[0, 0, 0, 0x4, 0, 0, 0, 0, 0];

/// The first frame of `kind` in the capture. The client's own SETTINGS precedes
/// its ACK of the origin's, so first-of-kind is the one carrying the entries.
fn first(frames: &[Frame], kind: u8) -> &Frame {
    frames
        .iter()
        .find(|f| f.kind == kind)
        .expect("the capture holds a frame of this kind")
}

#[test]
fn the_first_request_headers_frame_is_the_declared_h2_residual() {
    // Read until the client has actually sent a request, not just a preface.
    let (port, rx) = spawn_origin(SERVER_SETTINGS, |f| f.iter().any(|x| x.kind == 0x1));
    // The origin never answers, so the fetch fails; the HEADERS it sent first is
    // the whole point, so the error is discarded.
    let _ = Transport::new(test_roots()).request_once(
        &format!("https://localhost:{port}/"),
        &[],
        MAX_BODY_BYTES,
        Duration::from_secs(5),
    );
    let captured = rx.recv().unwrap();
    let frames = frames(&captured);
    let headers = first(&frames, 0x1);
    let h2 = FIREFOX_140_ESR.h2;

    // Residual 1 — stream id. `h2` opens client streams at 1; the persona's
    // first request rides stream 3 (§4.1), because Firefox opens a priority tree
    // first. Asserted against frot's own emission so it cannot drift silently.
    assert_eq!(headers.stream, 1, "h2 opens client streams at 1");
    assert_ne!(
        headers.stream, h2.initial_stream_id,
        "persona stream {} is a declared residual (§7 stage C)",
        h2.initial_stream_id
    );

    // Residual 2 — the PRIORITY flag. Firefox sets it with weight 42 /
    // depends_on 0 / exclusive 0; `h2` exposes no client PRIORITY API at all,
    // so the flag is absent. The persona weight is named, not silently dropped.
    assert_eq!(
        headers.flags & PRIORITY_FLAG,
        0,
        "no PRIORITY flag; persona weight {} is a declared residual (§7 stage C)",
        h2.priority_weight
    );

    // Residual 3 — pseudo-header order. `h2`'s `Pseudo` struct is hardcoded
    // `m,s,a,p`; the persona sends `m,p,a,s`. This literal is the *expected value
    // of the residual* (§12: "Stage B's `m,s,a,p` is written into the oracle as
    // the expected value with a pointer to §7 stage C"), decoded here out of the
    // real HPACK block rather than restated from a declaration.
    let observed = pseudo_order(&headers.payload);
    assert_eq!(observed, "m,s,a,p", "h2's hardcoded pseudo order");
    assert_ne!(
        observed, h2.pseudo_order,
        "persona order {} is a declared residual (§7 stage C)",
        h2.pseudo_order
    );

    // The composed akamai-h2 fingerprint, computed from this one capture and
    // from the profile by the same function — neither is stored, which is what
    // §12's "hashes are assertions, not fixtures" requires of the h2 layer.
    let window_update = u32::from_be_bytes(first(&frames, 0x8).payload[..4].try_into().unwrap());
    let wire = akamai(
        &settings(&first(&frames, 0x4).payload),
        window_update,
        "0", // akamai's notation for "no PRIORITY frames" — see residual 2.
        &observed,
    );
    let persona = persona_akamai();
    assert_ne!(wire, persona, "three of four fields are residuals (§6.5)");
    // The field that *does* match exactly, and the only one owning the pool
    // would not have changed (§6.5): the WINDOW_UPDATE increment.
    let field = |fp: &str| fp.split('|').nth(1).unwrap().to_string();
    assert_eq!(field(&wire), field(&persona));
    assert_eq!(field(&wire), h2.connection_window_increment.to_string());
}
