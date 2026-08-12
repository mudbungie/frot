//! The Web Audio persona SSOT, asserted directly (no JS, no starvation edges) —
//! the `audio.js` prelude reads exactly this `facts()` object through
//! `__frot_env_profile`, and the golden probe proves the JS end coheres.

use super::{facts, FIREFOX_AUDIO};

#[test]
fn sample_rate_is_the_pinned_low_entropy_rate() {
    // 44.1 kHz is PINNED, not measured: the real rate follows the output device
    // (measured 48000 on the capture box, both ESR lines — `bl-b128`), so it is
    // host entropy and determinism decides it. The fingerprint's whole
    // coordinate system (length, latency) is coherent against whatever it says.
    let v = facts();
    assert_eq!(v["sampleRate"], 44_100);
    assert_eq!(FIREFOX_AUDIO.sample_rate, 44_100);
}

#[test]
fn latencies_are_derived_from_the_sample_rate() {
    // baseLatency/outputLatency are frames/sampleRate — one home for the rate, so
    // they can never drift from it. Covers both derivations.
    let v = facts();
    let rate = f64::from(FIREFOX_AUDIO.sample_rate);
    assert_eq!(
        v["baseLatency"],
        f64::from(FIREFOX_AUDIO.base_latency_frames) / rate
    );
    assert_eq!(
        v["outputLatency"],
        f64::from(FIREFOX_AUDIO.output_latency_frames) / rate
    );
    // `baseLatency` is 0 because a real Linux Firefox reports 0 — measured on
    // both ESR lines with audio actually flowing, not assumed (`bl-b128`).
    assert_eq!(v["baseLatency"].as_f64().unwrap(), 0.0);
    // `outputLatency` is the device buffer: positive, and inside the 33.6–43.3 ms
    // band the same probe measured, so the pinned constant stays one a real
    // client was seen to report.
    let out = v["outputLatency"].as_f64().unwrap();
    assert!(
        out > 0.033 && out < 0.044,
        "outputLatency {out} left the band"
    );
}

#[test]
fn seed_is_a_fixed_non_zero_const() {
    // A FIXED seed (never host entropy) is what makes the rendered buffer match
    // across invocations; zero would collapse the xorshift expansion to silence.
    let v = facts();
    assert_eq!(v["audioSeed"], FIREFOX_AUDIO.audio_seed);
    assert_ne!(FIREFOX_AUDIO.audio_seed, 0);
}

#[test]
fn destination_is_a_low_entropy_stereo_default() {
    // A stereo output device — the common, low-entropy coherent choice for a
    // realtime context (an offline context reports its own channel count JS-side).
    let v = facts();
    assert_eq!(v["maxChannelCount"], 2);
    assert_eq!(FIREFOX_AUDIO.max_channel_count, 2);
}

#[test]
fn const_is_the_single_source() {
    // The struct fields the serializer reads are the const — one home per fact.
    assert_eq!(FIREFOX_AUDIO.base_latency_frames, 0);
    assert_eq!(FIREFOX_AUDIO.output_latency_frames, 1536);
}
