//! The h2 preface oracle (identity.md §12) — what frot actually puts on the wire
//! in its SETTINGS frame and the connection-level WINDOW_UPDATE that follows it.
//!
//! A throwaway-CA TLS origin negotiating `h2` by ALPN reads the client's raw
//! preface bytes instead of speaking HTTP, so the assertions below pin the exact
//! SETTINGS *set, order and values* — the akamai-h2 fingerprint's first two
//! fields. Every entry is either a profile fact frot enforces or a declared
//! residual named in the assertion, so neither can drift silently.
//!
//! The origin here stays **silent** (it sends no SETTINGS of its own), which is
//! what keeps the frame indices below stable: the client's first two frames are
//! its own. The request-side half of the fingerprint needs a replying origin and
//! lives in `h2_request.rs`.

use std::time::Duration;

use super::h2_wire::{frames, settings, spawn_origin, PREFACE};
use super::Transport;
use crate::fetch::profile::FIREFOX_140_ESR;
use crate::fetch::MAX_BODY_BYTES;

#[test]
fn the_h2_preface_carries_the_pinned_settings_and_window_update() {
    // Read until both of the client's own frames are in hand.
    let (port, rx) = spawn_origin(b"", |f| f.len() >= 2);
    // The origin never answers the request, so the fetch fails; the preface it
    // sent first is the whole point, so the error is discarded.
    let _ = Transport::new(super::h2_wire::test_roots()).request_once(
        &format!("https://localhost:{port}/"),
        &[],
        MAX_BODY_BYTES,
        Duration::from_secs(5),
    );
    let captured = rx.recv().unwrap();
    assert_eq!(&captured[..PREFACE.len()], PREFACE);
    let frames = frames(&captured);

    // Frame 1: SETTINGS. The set, its order and its values, expectations derived
    // from the profile so a persona edit moves the test with it. Firefox 140esr
    // sends `1:65536; 2:0; 4:131072; 5:16384`; the two differences are the §12
    // declared residuals asserted below, and nothing else may appear.
    let h2 = FIREFOX_140_ESR.h2;
    assert_eq!(frames[0].kind, 0x4, "first frame is SETTINGS");
    assert_eq!(
        settings(&frames[0].payload),
        vec![
            (2, u32::from(h2.enable_push)),
            (4, h2.initial_window_size),
            (5, h2.max_frame_size),
            // Residual: hyper always sends MAX_HEADER_LIST_SIZE at its default
            // (its config types the value `u32`, not `Option<u32>`), and Firefox
            // sends no id 6 at all. Pinned against frot's own emission so a
            // `cargo update` that changes hyper's default fails here.
            (6, 16_384),
        ]
    );
    // Residual: HEADER_TABLE_SIZE (id 1) is absent although the persona declares
    // it, because `hyper-util`'s pooled client builder has no setter for it.
    assert!(
        !settings(&frames[0].payload).iter().any(|&(id, _)| id == 1),
        "id 1 unreachable, persona value {} is a declared residual",
        h2.header_table_size
    );

    // Frame 2: the connection-level WINDOW_UPDATE, on stream 0. The *increment*
    // is the akamai-h2 fingerprint's second field and matches Firefox exactly.
    assert_eq!(frames[1].kind, 0x8, "second frame is WINDOW_UPDATE");
    assert_eq!(frames[1].stream, 0, "connection-level, not per-stream");
    assert_eq!(
        u32::from_be_bytes(frames[1].payload[..4].try_into().unwrap()),
        h2.connection_window_increment
    );
}
