use super::*;

fn j(html: &str) -> Value {
    dom_json(&Document::parse(html))
}

#[test]
fn empty_string_produces_skeleton_array() {
    let v = j("");
    assert!(v.is_array());
    assert!(!v.as_array().unwrap().is_empty());
}

#[test]
fn elements_serialize_with_type_name_attrs_children() {
    let v = j("<a href='/x'>hi</a>");
    let s = serde_json::to_string(&v).unwrap();
    assert!(s.contains("\"type\":\"element\""));
    assert!(s.contains("\"name\":\"a\""));
    assert!(s.contains("\"href\":\"/x\""));
    assert!(s.contains("\"type\":\"text\""));
    assert!(s.contains("\"value\":\"hi\""));
}

#[test]
fn comment_nodes_serialize() {
    let v = j("<!doctype html><html><body><!-- note --></body></html>");
    let s = serde_json::to_string(&v).unwrap();
    assert!(s.contains("\"type\":\"comment\""));
    assert!(s.contains("note"));
}

#[test]
fn doctype_node_serializes() {
    let v = j("<!doctype html><html></html>");
    let s = serde_json::to_string(&v).unwrap();
    assert!(s.contains("\"type\":\"doctype\""));
}

#[test]
fn attrs_object_is_empty_when_no_attributes() {
    let v = j("<p>hi</p>");
    let s = serde_json::to_string(&v).unwrap();
    assert!(s.contains("\"attrs\":{}"));
}

#[test]
fn children_array_is_empty_for_leaf_element() {
    let v = j("<br>");
    let s = serde_json::to_string(&v).unwrap();
    assert!(s.contains("\"name\":\"br\""));
    assert!(s.contains("\"children\":[]"));
}

#[test]
fn nested_elements_nest_in_children_arrays() {
    let v = j("<ul><li>a</li><li>b</li></ul>");
    let s = serde_json::to_string(&v).unwrap();
    let occurrences = s.matches("\"name\":\"li\"").count();
    assert_eq!(occurrences, 2);
}

#[test]
fn attrs_preserve_order_in_object_key_iteration() {
    let v = j("<a id='1' class='c' href='/' rel='ext'>x</a>");
    let s = serde_json::to_string(&v).unwrap();
    let i_id = s.find("\"id\"").unwrap();
    let i_class = s.find("\"class\"").unwrap();
    let i_href = s.find("\"href\"").unwrap();
    let i_rel = s.find("\"rel\"").unwrap();
    assert!(i_id < i_class && i_class < i_href && i_href < i_rel);
}

#[test]
fn returns_array_at_top_level() {
    let v = j("<html><body></body></html>");
    assert!(matches!(v, Value::Array(_)));
}
