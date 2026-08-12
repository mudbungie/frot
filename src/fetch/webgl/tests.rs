//! The WebGL persona SSOT, asserted directly (no JS, no starvation edges) — the
//! `webgl.js` prelude reads exactly this `facts()` object through
//! `__frot_env_profile`, and the golden probe proves the JS end coheres.

use super::{facts, FIREFOX_WEBGL};

#[test]
fn the_renderer_is_one_generalized_string_in_both_slots() {
    // Measured on both ESR lines (`bl-b128`, identity.md §3.11): Firefox masks
    // the VENDOR to "Mozilla" but reports the SAME generalized renderer class in
    // `RENDERER` and in the debug extension's `UNMASKED_RENDERER_WEBGL`. Pinning
    // them from one field is what makes the two impossible to disagree (I1).
    let v = facts();
    assert_eq!(v["maskedVendor"], "Mozilla");
    assert_eq!(v["unmaskedVendor"], "Mesa");
    assert_eq!(v["maskedRenderer"], v["unmaskedRenderer"]);
    assert_eq!(v["maskedRenderer"], "llvmpipe, or similar");
    // Linux-coherent, software: the renderer is llvmpipe, never hardware, and it
    // carries no driver/LLVM version — Gecko generalizes that away.
    let r = v["unmaskedRenderer"].as_str().unwrap();
    assert!(r.starts_with("llvmpipe") && r.ends_with(", or similar"));
    assert!(!r.contains("LLVM"));
}

#[test]
fn version_and_glsl_are_firefox_clean() {
    let v = facts();
    assert_eq!(v["version1"], "WebGL 1.0");
    assert_eq!(v["version2"], "WebGL 2.0");
    assert_eq!(v["glsl1"], "WebGL GLSL ES 1.0");
    assert_eq!(v["glsl2"], "WebGL GLSL ES 3.00");
}

#[test]
fn params_serialize_scalars_and_pairs() {
    // Covers BOTH Param arms: a scalar limit is a number, a range/dims is a
    // two-element array the JS side wraps in a typed array.
    let v = facts();
    assert_eq!(v["params1"]["MAX_TEXTURE_SIZE"], 16384);
    assert_eq!(v["params1"]["MAX_VERTEX_ATTRIBS"], 16);
    assert_eq!(v["params1"]["DEPTH_BITS"], 24);
    assert_eq!(
        v["params1"]["MAX_VIEWPORT_DIMS"],
        serde_json::json!([16384, 16384])
    );
    // 153esr clamps the line-width range to 1 where 140esr reported 1..=255 —
    // the one WebGL fact that moved between the lines, so it is pinned.
    assert_eq!(
        v["params1"]["ALIASED_LINE_WIDTH_RANGE"],
        serde_json::json!([1, 1])
    );
    // A WebGL2-only limit rides the separate table, and `MAX_SAMPLES` is one of
    // them: a real WebGL1 context exposes no such parameter at all.
    assert_eq!(v["params2"]["MAX_3D_TEXTURE_SIZE"], 2048);
    assert_eq!(v["params2"]["MAX_DRAW_BUFFERS"], 8);
    assert_eq!(v["params2"]["MAX_SAMPLES"], 8);
    assert!(v["params1"]["MAX_SAMPLES"].is_null());
}

#[test]
fn extension_lists_are_coherent_with_software_llvmpipe() {
    let v = facts();
    let e1: Vec<String> = serde_json::from_value(v["extensions1"].clone()).unwrap();
    let e2: Vec<String> = serde_json::from_value(v["extensions2"].clone()).unwrap();
    // The debug-renderer extension MUST be present or UNMASKED_* is unreachable.
    assert!(e1.contains(&"WEBGL_debug_renderer_info".to_string()));
    assert!(e2.contains(&"WEBGL_debug_renderer_info".to_string()));
    // Anisotropy is present and coherent with the modern Mesa build + the
    // MAX_TEXTURE_MAX_ANISOTROPY_EXT limit.
    assert!(e1.contains(&"EXT_texture_filter_anisotropic".to_string()));
    assert_eq!(v["params1"]["MAX_TEXTURE_MAX_ANISOTROPY_EXT"], 16);
    // ASTC was assumed absent here ("a mobile-GPU signal llvmpipe does not
    // support") until `bl-b128` measured it: this Mesa's llvmpipe advertises
    // `WEBGL_compressed_texture_astc` on both contexts. The measurement wins
    // over the belief — that is the whole point of re-reading it.
    assert!(e1.contains(&"WEBGL_compressed_texture_astc".to_string()));
    assert!(e2.contains(&"WEBGL_compressed_texture_astc".to_string()));
    // WebGL2 promotes many WebGL1 extensions to core, so they drop from its list.
    assert!(!e2.contains(&"ANGLE_instanced_arrays".to_string()));
    assert!(!e2.contains(&"OES_vertex_array_object".to_string()));
}

#[test]
fn const_is_the_single_source() {
    // The struct fields the serializer reads are the const — one home for the fact.
    assert_eq!(FIREFOX_WEBGL.masked_vendor, "Mozilla");
    assert_eq!(FIREFOX_WEBGL.version.0, "WebGL 1.0");
    assert_eq!(FIREFOX_WEBGL.params1.len(), 20);
    assert_eq!(FIREFOX_WEBGL.params2.len(), 9);
}
