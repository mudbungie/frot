//! The WebGL fingerprint persona — SSOT for `bl-f624` (identity.md §10/§11,
//! js.md §7). A companion to [`super::profile`]'s [`FIREFOX_153_ESR`]: the
//! browser persona's WebGL facts, kept out of `profile.rs` only because that file
//! sits at the source-line cap. Delivered through the SAME one channel
//! (`__frot_env_profile`, `env.rs`) so the JS `webgl.js` prelude carries no
//! identity literal of its own (I1), exactly as `canvas.js` reads `canvas_seed`.
//!
//! Every value here is a COHERENT, DETERMINISTIC function of the pinned persona
//! (Firefox 153 ESR, **Linux x86_64**, software rendering), and every one of them
//! was **read from the running binary** (`bl-b128`, 2026-08-11, identity.md
//! §3.11) — Firefox 153.0esr and 140.12.0esr driven over Marionette on one box
//! with Mesa forced to llvmpipe, two runs each.
//!
//! ## Firefox generalizes the renderer; it does not mask it to `"Mozilla"`
//!
//! Measured, on **both** ESR lines: `getParameter(VENDOR)` is `"Mozilla"`, but
//! `getParameter(RENDERER)` and `WEBGL_debug_renderer_info`'s
//! `UNMASKED_RENDERER_WEBGL` are the SAME **generalized** string — `"llvmpipe, or
//! similar"` under software rendering, `"<GPU class>, or similar"` on hardware.
//! Gecko buckets the driver string into a class and appends `, or similar`; the
//! driver/LLVM version detail never reaches content at all. So the renderer is
//! **one fact with one home** here, not a masked/unmasked pair, and no Mesa or
//! LLVM version is written down — there is none to write.
//!
//! Software rendering stays the persona choice (it never over-claims hardware,
//! and it is common for headless Linux Firefox); what changed is that the choice
//! now costs no version coupling.
//!
//! [`FIREFOX_153_ESR`]: super::FIREFOX_153_ESR

use serde_json::{json, Value};

/// One `getParameter` value: a scalar limit, or a two-element range/dims pair
/// (`MAX_VIEWPORT_DIMS`, the `ALIASED_*` ranges). The JS side decides the JS
/// wrapper (a plain number, or an `Int32Array`/`Float32Array` by parameter name).
#[derive(Debug, Clone, Copy)]
pub enum Param {
    /// A scalar integer limit (`MAX_TEXTURE_SIZE`, `*_BITS`, …).
    N(i64),
    /// A two-element pair (`MAX_VIEWPORT_DIMS`, `ALIASED_LINE_WIDTH_RANGE`, …).
    Pair(i64, i64),
}

/// The WebGL facts of one browser persona (see module docs). Pure `const` data;
/// the strings are Firefox's, the limits are one real llvmpipe build's.
#[derive(Debug, Clone, Copy)]
pub struct WebglProfile {
    /// `getParameter(VENDOR)` — Firefox masks the vendor to this literal
    /// regardless of the real GPU. Measured `"Mozilla"` on both ESR lines.
    pub masked_vendor: &'static str,
    /// `WEBGL_debug_renderer_info`'s `UNMASKED_VENDOR_WEBGL` — the real GL vendor.
    pub unmasked_vendor: &'static str,
    /// `getParameter(RENDERER)` **and** `UNMASKED_RENDERER_WEBGL`: Gecko reports
    /// one generalized string in both slots (see module docs), so this is one
    /// field rather than the masked/unmasked pair it used to be stored as.
    pub renderer: &'static str,
    /// `getParameter(VERSION)` for WebGL1 / WebGL2 (`"WebGL 1.0"` / `"WebGL 2.0"`).
    pub version: (&'static str, &'static str),
    /// `getParameter(SHADING_LANGUAGE_VERSION)` for WebGL1 / WebGL2.
    pub glsl: (&'static str, &'static str),
    /// WebGL1 `getParameter` limits, keyed by the standard constant name.
    pub params1: &'static [(&'static str, Param)],
    /// WebGL2-only additional limits (the WebGL1 set still applies).
    pub params2: &'static [(&'static str, Param)],
    /// WebGL1 `getSupportedExtensions()`, in the order Firefox reports them.
    pub extensions1: &'static [&'static str],
    /// WebGL2 `getSupportedExtensions()` (the promoted-to-core entries drop out).
    pub extensions2: &'static [&'static str],
}

/// The WebGL persona: **measured from Firefox 153.0esr** (Linux x86_64, Mesa
/// llvmpipe via `LIBGL_ALWAYS_SOFTWARE`) on 2026-08-11, `bl-b128`. Every string,
/// limit and extension below was read from the running binary; the same probe on
/// 140.12.0esr agrees on all of it but `ALIASED_LINE_WIDTH_RANGE` (identity.md
/// §3.11), which is the one WebGL fact that moved between the two lines.
pub const FIREFOX_WEBGL: WebglProfile = WebglProfile {
    masked_vendor: "Mozilla",
    unmasked_vendor: "Mesa",
    // Gecko's generalized renderer class — no Mesa/LLVM version, by design.
    renderer: "llvmpipe, or similar",
    version: ("WebGL 1.0", "WebGL 2.0"),
    glsl: ("WebGL GLSL ES 1.0", "WebGL GLSL ES 3.00"),
    params1: &[
        ("MAX_TEXTURE_SIZE", Param::N(16384)),
        ("MAX_CUBE_MAP_TEXTURE_SIZE", Param::N(16384)),
        ("MAX_RENDERBUFFER_SIZE", Param::N(16384)),
        ("MAX_VIEWPORT_DIMS", Param::Pair(16384, 16384)),
        ("MAX_VERTEX_ATTRIBS", Param::N(16)),
        ("MAX_VERTEX_UNIFORM_VECTORS", Param::N(4096)),
        ("MAX_FRAGMENT_UNIFORM_VECTORS", Param::N(4096)),
        ("MAX_VARYING_VECTORS", Param::N(32)),
        ("MAX_VERTEX_TEXTURE_IMAGE_UNITS", Param::N(32)),
        ("MAX_TEXTURE_IMAGE_UNITS", Param::N(32)),
        ("MAX_COMBINED_TEXTURE_IMAGE_UNITS", Param::N(160)),
        // 153esr clamps line width to 1; 140esr reported `1..=255` on the same
        // box and driver — the one measured 140→153 WebGL difference.
        ("ALIASED_LINE_WIDTH_RANGE", Param::Pair(1, 1)),
        ("ALIASED_POINT_SIZE_RANGE", Param::Pair(1, 256)),
        ("MAX_TEXTURE_MAX_ANISOTROPY_EXT", Param::N(16)),
        ("RED_BITS", Param::N(8)),
        ("GREEN_BITS", Param::N(8)),
        ("BLUE_BITS", Param::N(8)),
        ("ALPHA_BITS", Param::N(8)),
        ("DEPTH_BITS", Param::N(24)),
        // Measured 0: the default llvmpipe framebuffer carries no stencil.
        ("STENCIL_BITS", Param::N(0)),
    ],
    params2: &[
        // WebGL1 exposes no `MAX_SAMPLES` at all (it is core in WebGL2 only), so
        // it lives here — the previous table offered it to WebGL1 callers.
        ("MAX_SAMPLES", Param::N(8)),
        ("MAX_3D_TEXTURE_SIZE", Param::N(2048)),
        ("MAX_ARRAY_TEXTURE_LAYERS", Param::N(2048)),
        ("MAX_DRAW_BUFFERS", Param::N(8)),
        ("MAX_COLOR_ATTACHMENTS", Param::N(8)),
        ("MAX_VERTEX_UNIFORM_BLOCKS", Param::N(15)),
        ("MAX_FRAGMENT_UNIFORM_BLOCKS", Param::N(15)),
        ("MAX_UNIFORM_BUFFER_BINDINGS", Param::N(120)),
        ("MAX_TEXTURE_LOD_BIAS", Param::N(16)),
    ],
    extensions1: &[
        "ANGLE_instanced_arrays",
        "EXT_blend_minmax",
        "EXT_color_buffer_half_float",
        "EXT_depth_clamp",
        "EXT_float_blend",
        "EXT_frag_depth",
        "EXT_shader_texture_lod",
        "EXT_sRGB",
        "EXT_texture_compression_bptc",
        "EXT_texture_compression_rgtc",
        "EXT_texture_filter_anisotropic",
        "OES_element_index_uint",
        "OES_fbo_render_mipmap",
        "OES_standard_derivatives",
        "OES_texture_float",
        "OES_texture_float_linear",
        "OES_texture_half_float",
        "OES_texture_half_float_linear",
        "OES_vertex_array_object",
        "WEBGL_color_buffer_float",
        "WEBGL_compressed_texture_astc",
        "WEBGL_compressed_texture_etc",
        "WEBGL_compressed_texture_s3tc",
        "WEBGL_compressed_texture_s3tc_srgb",
        "WEBGL_debug_renderer_info",
        "WEBGL_debug_shaders",
        "WEBGL_depth_texture",
        "WEBGL_draw_buffers",
        "WEBGL_lose_context",
    ],
    extensions2: &[
        "EXT_color_buffer_float",
        "EXT_depth_clamp",
        "EXT_float_blend",
        "EXT_texture_compression_bptc",
        "EXT_texture_compression_rgtc",
        "EXT_texture_filter_anisotropic",
        "OES_draw_buffers_indexed",
        "OES_texture_float_linear",
        "OVR_multiview2",
        "WEBGL_compressed_texture_astc",
        "WEBGL_compressed_texture_etc",
        "WEBGL_compressed_texture_s3tc",
        "WEBGL_compressed_texture_s3tc_srgb",
        "WEBGL_debug_renderer_info",
        "WEBGL_debug_shaders",
        "WEBGL_lose_context",
    ],
};

/// Serialize one parameter table to a `{name: value}` JSON object. A [`Param::N`]
/// becomes a number, a [`Param::Pair`] a two-element array — the JS side wraps the
/// array in the right typed-array class by name (§ `webgl.js`).
fn params(table: &[(&str, Param)]) -> Value {
    let mut m = serde_json::Map::new();
    for (name, v) in table {
        let value = match *v {
            Param::N(n) => json!(n),
            Param::Pair(a, b) => json!([a, b]),
        };
        m.insert((*name).to_string(), value);
    }
    Value::Object(m)
}

/// The `webgl` sub-object of `__frot_env_profile` (`env.rs` embeds it). The single
/// derivation site for every JS-visible WebGL fact, so `webgl.js` holds none.
pub fn facts() -> Value {
    let p = &FIREFOX_WEBGL;
    json!({
        "maskedVendor": p.masked_vendor,
        "maskedRenderer": p.renderer,
        "unmaskedVendor": p.unmasked_vendor,
        "unmaskedRenderer": p.renderer,
        "version1": p.version.0,
        "version2": p.version.1,
        "glsl1": p.glsl.0,
        "glsl2": p.glsl.1,
        "params1": params(p.params1),
        "params2": params(p.params2),
        "extensions1": p.extensions1,
        "extensions2": p.extensions2,
    })
}

#[cfg(test)]
mod tests;
