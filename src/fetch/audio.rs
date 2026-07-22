//! The Web Audio fingerprint persona — SSOT for `bl-8733` (identity.md §10/§11,
//! js.md §7). A companion to [`super::profile`]'s [`FIREFOX_140_ESR`] and a
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
//! platform's audio-DSP float behaviour. Firefox 140 ESR **exposes** the whole
//! `AudioContext`/`OfflineAudioContext` surface, so returning `undefined` (frot's
//! prior state) is a LOUDER tell than a costume (§10). Under Mark's 2026-07-20
//! ruling the masquerade is in scope PROVIDED every value is a DETERMINISTIC
//! function of this const, stable across invocations — a random-per-run or
//! persona-contradicting sample set would be the louder tell absence was.
//!
//! ## The chosen Firefox-140-ESR-on-Linux defaults (all coherent, low-entropy)
//!
//! * `sample_rate` **44100** — Firefox's default `AudioContext.sampleRate`.
//! * `base_latency` / `output_latency` — derived `frames / sample_rate`, so the
//!   one `sample_rate` fact is their single source; small positive values, the
//!   shape a realtime context reports (an offline context reports `0`, computed
//!   JS-side, not here).
//! * `max_channel_count` **2** — a stereo output device, the common low-entropy
//!   default; an offline context instead reports its own channel count (JS-side).
//! * `audio_seed` — an opaque FIXED digest seed (never host entropy) so the
//!   rendered buffer is a deterministic function of `(seed + graph digest)`,
//!   matching across invocations exactly like `canvas_seed`/the webgl seed.
//!
//! The rendered samples are a digest expansion in the plausible `[-1, 1]` audio
//! range — waveform realism against a real Gecko DSP render is the declared §11
//! residual, exactly as pixel realism is for canvas/WebGL; determinism and
//! coherence are what a fingerprint reads.
//!
//! [`FIREFOX_140_ESR`]: super::FIREFOX_140_ESR

use serde_json::{json, Value};

/// The Web Audio facts of one browser persona (see module docs). Pure `const`
/// data; latencies are stored as frame counts so `sample_rate` is their SSOT.
#[derive(Debug, Clone, Copy)]
pub struct AudioProfile {
    /// `BaseAudioContext.sampleRate` — Firefox's default output rate.
    pub sample_rate: u32,
    /// Opaque FIXED seed for the rendered-buffer digest (never host entropy), so
    /// `startRendering()`'s float samples match across invocations.
    pub audio_seed: u32,
    /// `AudioContext.baseLatency` numerator, in frames: `baseLatency` is
    /// `base_latency_frames / sample_rate` (one render quantum's worth).
    pub base_latency_frames: u32,
    /// `AudioContext.outputLatency` numerator, in frames (device buffering).
    pub output_latency_frames: u32,
    /// A realtime `AudioDestinationNode.maxChannelCount` — a stereo device.
    pub max_channel_count: u32,
}

/// The pinned audio persona: Firefox 140 ESR on Linux x86_64, 44.1 kHz stereo.
pub const FIREFOX_AUDIO: AudioProfile = AudioProfile {
    sample_rate: 44_100,
    // Opaque fixed seed; mixed with the graph digest, never surfaced raw.
    audio_seed: 0x_a0d1_05ee,
    // 128 frames — Web Audio's render-quantum size; a coherent baseLatency floor.
    base_latency_frames: 128,
    // 512 frames — a plausible device output buffer at 44.1 kHz (~11.6 ms).
    output_latency_frames: 512,
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
