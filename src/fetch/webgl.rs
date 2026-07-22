//! The WebGL fingerprint persona — SSOT for `bl-f624` (identity.md §10/§11,
//! js.md §7). A companion to [`super::profile`]'s [`FIREFOX_140_ESR`]: the
//! browser persona's WebGL facts, kept out of `profile.rs` only because that file
//! sits at the source-line cap. Delivered through the SAME one channel
//! (`__frot_env_profile`, `env.rs`) so the JS `webgl.js` prelude carries no
//! identity literal of its own (I1), exactly as `canvas.js` reads `canvas_seed`.
//!
//! Every value here is a COHERENT, DETERMINISTIC function of the pinned persona
//! (Firefox 140 ESR, **Linux x86_64**, software rendering). `VENDOR`/`RENDERER`
//! are Firefox's masked `"Mozilla"`; the real GPU strings surface only through
//! the `WEBGL_debug_renderer_info` extension and name **Mesa llvmpipe** — a
//! software rasteriser that is common for headless Linux Firefox and NEVER
//! over-claims specific hardware (the safest coherent choice under Mark's ruling:
//! an NVIDIA/Intel string on a software persona would be a louder tell than
//! absence). The `LLVM 19.1.7` build, the anisotropy support, and the extension
//! set all track ONE real Mesa 24.2 generation (Ubuntu 24.04.x) so nothing here
//! can contradict anything else here.
//!
//! [`FIREFOX_140_ESR`]: super::FIREFOX_140_ESR

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
    /// `getParameter(VENDOR)` / `getParameter(RENDERER)` — Firefox masks BOTH to
    /// this literal regardless of the real GPU (the anti-fingerprint default).
    pub masked: &'static str,
    /// `WEBGL_debug_renderer_info`'s `UNMASKED_VENDOR_WEBGL` — the real GL vendor.
    pub unmasked_vendor: &'static str,
    /// `UNMASKED_RENDERER_WEBGL` — the real GL renderer (Mesa llvmpipe, software).
    pub unmasked_renderer: &'static str,
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

/// The pinned WebGL persona: Firefox 140 ESR on Linux x86_64, Mesa 24.2 llvmpipe
/// (LLVM 19.1.7). Captured against browserleaks/webgl behaviour + Mesa docs.
pub const FIREFOX_WEBGL: WebglProfile = WebglProfile {
    masked: "Mozilla",
    unmasked_vendor: "Mesa",
    unmasked_renderer: "llvmpipe (LLVM 19.1.7, 256 bits)",
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
        ("MAX_VERTEX_TEXTURE_IMAGE_UNITS", Param::N(16)),
        ("MAX_TEXTURE_IMAGE_UNITS", Param::N(16)),
        ("MAX_COMBINED_TEXTURE_IMAGE_UNITS", Param::N(48)),
        ("ALIASED_LINE_WIDTH_RANGE", Param::Pair(1, 255)),
        ("ALIASED_POINT_SIZE_RANGE", Param::Pair(1, 255)),
        ("MAX_TEXTURE_MAX_ANISOTROPY_EXT", Param::N(16)),
        ("RED_BITS", Param::N(8)),
        ("GREEN_BITS", Param::N(8)),
        ("BLUE_BITS", Param::N(8)),
        ("ALPHA_BITS", Param::N(8)),
        ("DEPTH_BITS", Param::N(24)),
        ("STENCIL_BITS", Param::N(8)),
        ("MAX_SAMPLES", Param::N(4)),
    ],
    params2: &[
        ("MAX_3D_TEXTURE_SIZE", Param::N(2048)),
        ("MAX_ARRAY_TEXTURE_LAYERS", Param::N(2048)),
        ("MAX_DRAW_BUFFERS", Param::N(8)),
        ("MAX_COLOR_ATTACHMENTS", Param::N(8)),
        ("MAX_VERTEX_UNIFORM_BLOCKS", Param::N(14)),
        ("MAX_FRAGMENT_UNIFORM_BLOCKS", Param::N(14)),
        ("MAX_UNIFORM_BUFFER_BINDINGS", Param::N(84)),
        ("MAX_TEXTURE_LOD_BIAS", Param::N(16)),
    ],
    extensions1: &[
        "ANGLE_instanced_arrays",
        "EXT_blend_minmax",
        "EXT_color_buffer_half_float",
        "EXT_disjoint_timer_query",
        "EXT_float_blend",
        "EXT_frag_depth",
        "EXT_sRGB",
        "EXT_shader_texture_lod",
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
        "WEBGL_compressed_texture_etc",
        "WEBGL_compressed_texture_etc1",
        "WEBGL_compressed_texture_s3tc",
        "WEBGL_compressed_texture_s3tc_srgb",
        "WEBGL_debug_renderer_info",
        "WEBGL_debug_shaders",
        "WEBGL_depth_texture",
        "WEBGL_draw_buffers",
        "WEBGL_lose_context",
        "WEBGL_provoking_vertex",
    ],
    extensions2: &[
        "EXT_color_buffer_float",
        "EXT_color_buffer_half_float",
        "EXT_disjoint_timer_query_webgl2",
        "EXT_float_blend",
        "EXT_texture_compression_bptc",
        "EXT_texture_compression_rgtc",
        "EXT_texture_filter_anisotropic",
        "EXT_texture_norm16",
        "OES_draw_buffers_indexed",
        "OES_texture_float_linear",
        "OES_texture_half_float_linear",
        "OVR_multiview2",
        "WEBGL_clip_cull_distance",
        "WEBGL_compressed_texture_etc",
        "WEBGL_compressed_texture_etc1",
        "WEBGL_compressed_texture_s3tc",
        "WEBGL_compressed_texture_s3tc_srgb",
        "WEBGL_debug_renderer_info",
        "WEBGL_debug_shaders",
        "WEBGL_lose_context",
        "WEBGL_multi_draw",
        "WEBGL_provoking_vertex",
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
        "maskedVendor": p.masked,
        "maskedRenderer": p.masked,
        "unmaskedVendor": p.unmasked_vendor,
        "unmaskedRenderer": p.unmasked_renderer,
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
