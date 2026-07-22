//! The Web Audio persona SSOT, asserted directly (no JS, no starvation edges) —
//! the `audio.js` prelude reads exactly this `facts()` object through
//! `__frot_env_profile`, and the golden probe proves the JS end coheres.

use super::{facts, FIREFOX_AUDIO};

#[test]
fn sample_rate_is_firefox_default() {
    // 44.1 kHz is Firefox's default AudioContext.sampleRate; the fingerprint's
    // whole coordinate system (length, latency) is coherent only against it.
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
    // Both are small positive values (a realtime context's shape), never negative
    // or absurd: a plausible sub-20-ms audio latency.
    assert!(v["baseLatency"].as_f64().unwrap() > 0.0);
    assert!(v["baseLatency"].as_f64().unwrap() < 0.02);
    assert!(v["outputLatency"].as_f64().unwrap() > v["baseLatency"].as_f64().unwrap());
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
    assert_eq!(FIREFOX_AUDIO.base_latency_frames, 128);
    assert_eq!(FIREFOX_AUDIO.output_latency_frames, 512);
}
