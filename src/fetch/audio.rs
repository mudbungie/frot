//! The Web Audio fingerprint persona — SSOT for `bl-8733` (identity.md §10/§11,
//! js.md §7). A companion to [`super::profile`]'s [`FIREFOX_153_ESR`] and a
//! sibling to [`super::webgl`]: the browser persona's audio facts, kept out of
//! `profile.rs` only because that file sits at the source-line cap. Delivered
//! through the SAME one channel (`__frot_env_profile`, `env.rs`) so the JS
//! `audio.js` prelude carries no identity literal of its own (I1), exactly as
//! `canvas.js` reads `canvas_seed` and `webgl.js` reads the `webgl` sub-object.
//!
//! ## What the audio fingerprint measures, and why a coherent sim is in scope
//!
//! A fingerprinter builds an `OfflineAudioContext`, wires an `OscillatorNode`
//! through a `DynamicsCompressorNode` to the destination, `startRendering()`s,
//! and hashes the rendered `AudioBuffer`'s float samples — the hash reflects the
//! platform's audio-DSP float behaviour. Firefox ESR **exposes** the whole
//! `AudioContext`/`OfflineAudioContext` surface, so returning `undefined` (frot's
//! prior state) is a LOUDER tell than a costume (§10). Under Mark's 2026-07-20
//! ruling the masquerade is in scope PROVIDED every value is a DETERMINISTIC
//! function of this const, stable across invocations — a random-per-run or
//! persona-contradicting sample set would be the louder tell absence was.
//!
//! ## What is measured here, and what is pinned — they are not the same thing
//!
//! Re-read from the running binary (`bl-b128`, 2026-08-11, identity.md §3.11):
//! Firefox 153.0esr and 140.12.0esr on Linux, Marionette, autoplay unblocked so
//! the context actually reached `state: "running"` with a live output stream.
//! **The two lines agree on every value below** — no Web Audio fact moved
//! between them.
//!
//! * `max_channel_count` **2** — MEASURED, both lines: a stereo output device.
//! * `base_latency_frames` **0** — MEASURED, both lines, four runs: Linux
//!   Firefox reports `baseLatency === 0` even with audio flowing. The previous
//!   `128` (one render quantum) was a plausible-looking guess that no Firefox
//!   emits, so it went.
//! * `sample_rate` **44100** — PINNED, not measured: the rate follows the output
//!   *device* (this box's reports 48000 on both lines), so it is host entropy
//!   exactly like `hardwareConcurrency` (identity.md §8) and determinism decides
//!   it. There is no "Firefox default" to measure. 44.1 kHz is the common,
//!   low-entropy choice; the fingerprint's coordinate system is coherent against
//!   whatever this says.
//! * `output_latency_frames` **1536** — PINNED, inside the measured band: device
//!   buffering, and it moved run-to-run on one box (33.6–43.3 ms across four
//!   runs). A varying number cannot be pinned honestly, so frot pins a constant
//!   that a real client was actually seen to report — 1536/44100 = 34.8 ms.
//! * `audio_seed` — frot's own opaque FIXED digest seed (never host entropy, and
//!   nothing a browser exposes), so the rendered buffer is a deterministic
//!   function of `(seed + graph digest)`, like `canvas_seed`.
//!
//! The rendered samples are a digest expansion in the plausible `[-1, 1]` audio
//! range — waveform realism against a real Gecko DSP render is the declared §11
//! residual, exactly as pixel realism is for canvas/WebGL; determinism and
//! coherence are what a fingerprint reads.
//!
//! [`FIREFOX_153_ESR`]: super::FIREFOX_153_ESR

use serde_json::{json, Value};

/// The Web Audio facts of one browser persona (see module docs). Pure `const`
/// data; latencies are stored as frame counts so `sample_rate` is their SSOT.
#[derive(Debug, Clone, Copy)]
pub struct AudioProfile {
    /// `BaseAudioContext.sampleRate` — PINNED, not measured: the real rate follows
    /// the output device (see module docs).
    pub sample_rate: u32,
    /// Opaque FIXED seed for the rendered-buffer digest (never host entropy), so
    /// `startRendering()`'s float samples match across invocations.
    pub audio_seed: u32,
    /// `AudioContext.baseLatency` numerator, in frames: `baseLatency` is
    /// `base_latency_frames / sample_rate`. Measured `0` on Linux, both ESR lines.
    pub base_latency_frames: u32,
    /// `AudioContext.outputLatency` numerator, in frames (device buffering) —
    /// PINNED inside the measured band, since the real value moves per run.
    pub output_latency_frames: u32,
    /// A realtime `AudioDestinationNode.maxChannelCount` — a stereo device.
    /// Measured `2` on both ESR lines.
    pub max_channel_count: u32,
}

/// The audio persona. Read from Firefox 153.0esr on 2026-08-11 (`bl-b128`,
/// identity.md §3.11), cross-checked against 140.12.0esr on the same box: the
/// two lines agree. Each field's doc says whether it is MEASURED or PINNED —
/// the device-dependent ones cannot honestly be either measured or invented, so
/// they are pinned and labelled.
pub const FIREFOX_AUDIO: AudioProfile = AudioProfile {
    sample_rate: 44_100,
    // Opaque fixed seed; mixed with the graph digest, never surfaced raw.
    audio_seed: 0x_a0d1_05ee,
    // Measured 0 on Linux (both ESR lines, four runs, live output stream).
    base_latency_frames: 0,
    // 1536 frames — 34.8 ms at 44.1 kHz, inside the 33.6–43.3 ms band measured
    // on this box. Pinned because the real value varies per run.
    output_latency_frames: 1536,
    max_channel_count: 2,
};

/// The `audio` sub-object of `__frot_env_profile` (`env.rs` embeds it). The single
/// derivation site for every JS-visible audio fact, so `audio.js` holds none.
/// `baseLatency`/`outputLatency` are derived from `sample_rate` here (one home for
/// the rate); the JS side never re-computes them.
pub fn facts() -> Value {
    let p = &FIREFOX_AUDIO;
    let rate = f64::from(p.sample_rate);
    json!({
        "sampleRate": p.sample_rate,
        "audioSeed": p.audio_seed,
        "baseLatency": f64::from(p.base_latency_frames) / rate,
        "outputLatency": f64::from(p.output_latency_frames) / rate,
        "maxChannelCount": p.max_channel_count,
    })
}

#[cfg(test)]
mod tests;
