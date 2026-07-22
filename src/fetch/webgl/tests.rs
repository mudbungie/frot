//! The WebGL persona SSOT, asserted directly (no JS, no starvation edges) — the
//! `webgl.js` prelude reads exactly this `facts()` object through
//! `__frot_env_profile`, and the golden probe proves the JS end coheres.

use super::{facts, FIREFOX_WEBGL};

#[test]
fn masked_strings_never_leak_the_real_gpu() {
    // Firefox masks VENDOR and RENDERER to "Mozilla"; the real Mesa strings live
    // ONLY behind the debug-renderer extension — that separation is the whole
    // point (a masked slot naming a GPU would be the tell).
    let v = facts();
    assert_eq!(v["maskedVendor"], "Mozilla");
    assert_eq!(v["maskedRenderer"], "Mozilla");
    assert_eq!(v["unmaskedVendor"], "Mesa");
    assert_eq!(v["unmaskedRenderer"], "llvmpipe (LLVM 19.1.7, 256 bits)");
    // Linux-coherent, software: the unmasked renderer is llvmpipe, never hardware.
    assert!(v["unmaskedRenderer"]
        .as_str()
        .unwrap()
        .starts_with("llvmpipe"));
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
    assert_eq!(
        v["params1"]["ALIASED_LINE_WIDTH_RANGE"],
        serde_json::json!([1, 255])
    );
    // A WebGL2-only limit rides the separate table.
    assert_eq!(v["params2"]["MAX_3D_TEXTURE_SIZE"], 2048);
    assert_eq!(v["params2"]["MAX_DRAW_BUFFERS"], 8);
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
    // ASTC is a mobile-GPU signal llvmpipe does NOT support — its absence is part
    // of the coherence (present it and we'd contradict the software persona).
    assert!(!e1.iter().any(|x| x.contains("astc")));
    assert!(!e2.iter().any(|x| x.contains("astc")));
    // WebGL2 promotes many WebGL1 extensions to core, so they drop from its list.
    assert!(!e2.contains(&"ANGLE_instanced_arrays".to_string()));
    assert!(!e2.contains(&"OES_vertex_array_object".to_string()));
}

#[test]
fn const_is_the_single_source() {
    // The struct fields the serializer reads are the const — one home for the fact.
    assert_eq!(FIREFOX_WEBGL.masked, "Mozilla");
    assert_eq!(FIREFOX_WEBGL.version.0, "WebGL 1.0");
    assert_eq!(FIREFOX_WEBGL.params1.len(), 21);
    assert_eq!(FIREFOX_WEBGL.params2.len(), 8);
}
