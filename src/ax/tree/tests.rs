use super::*;

fn tree(html: &str) -> Value {
    ax_tree(&Document::parse(html))
}

fn find_first<'a>(v: &'a Value, role: &str) -> Option<&'a Value> {
    if let Value::Array(arr) = v {
        for item in arr {
            if item["role"] == role {
                return Some(item);
            }
            if let Some(found) = find_first(&item["children"], role) {
                return Some(found);
            }
        }
    }
    None
}

#[test]
fn empty_document_yields_empty_array() {
    let v = tree("");
    assert!(v.is_array());
    // html5ever still injects an html skeleton but every element collapses to generic.
    if let Value::Array(a) = &v {
        // body's children are empty
        assert!(a.is_empty(), "expected empty array, got {:?}", a);
    }
}

#[test]
fn h1_becomes_heading_node_with_level() {
    let v = tree("<h1>Hello</h1>");
    let h = find_first(&v, "heading").expect("heading node");
    assert_eq!(h["name"], "Hello");
    assert_eq!(h["level"], 1);
}

#[test]
fn h3_carries_level_3() {
    let v = tree("<h3>Sub</h3>");
    let h = find_first(&v, "heading").unwrap();
    assert_eq!(h["level"], 3);
}

#[test]
fn aria_level_overrides() {
    let v = tree("<h2 aria-level='5'>Deep</h2>");
    let h = find_first(&v, "heading").unwrap();
    assert_eq!(h["level"], 5);
}

#[test]
fn anchor_with_href_yields_link_with_text() {
    let v = tree("<a href='/x'>Home</a>");
    let a = find_first(&v, "link").unwrap();
    assert_eq!(a["name"], "Home");
}

#[test]
fn anchor_without_href_collapses_to_children() {
    let v = tree("<a><strong>Important</strong></a>");
    let s = find_first(&v, "strong").unwrap();
    assert_eq!(s["name"], "Important");
    assert!(find_first(&v, "link").is_none());
}

#[test]
fn nested_landmarks_preserved() {
    let v = tree("<main><nav>Nav</nav><article>Body</article></main>");
    let main = find_first(&v, "main").unwrap();
    let kids = main["children"].as_array().unwrap();
    let roles: Vec<&str> = kids.iter().filter_map(|n| n["role"].as_str()).collect();
    assert!(roles.contains(&"navigation"));
    assert!(roles.contains(&"article"));
}

#[test]
fn divs_collapse_promoting_children() {
    let v = tree("<div><div><button>Go</button></div></div>");
    let b = find_first(&v, "button").unwrap();
    assert_eq!(b["name"], "Go");
}

#[test]
fn presentation_role_collapses() {
    let v = tree("<div role='presentation'><h1>Title</h1></div>");
    let h = find_first(&v, "heading").unwrap();
    assert_eq!(h["name"], "Title");
}

#[test]
fn none_role_collapses() {
    let v = tree("<div role='none'><button>X</button></div>");
    assert!(find_first(&v, "button").is_some());
}

#[test]
fn explicit_generic_role_collapses() {
    let v = tree("<div role='generic'><h2>X</h2></div>");
    let h = find_first(&v, "heading").unwrap();
    assert_eq!(h["name"], "X");
}

#[test]
fn script_subtree_skipped_entirely() {
    let v = tree("<script><button>nope</button></script>");
    assert!(find_first(&v, "button").is_none());
}

#[test]
fn style_subtree_skipped() {
    let v = tree("<style><p>nope</p></style>");
    assert!(find_first(&v, "paragraph").is_none());
}

#[test]
fn template_subtree_skipped() {
    let v = tree("<template><button>nope</button></template>");
    assert!(find_first(&v, "button").is_none());
}

#[test]
fn noscript_subtree_skipped() {
    let v = tree("<noscript><h1>Old</h1></noscript>");
    assert!(find_first(&v, "heading").is_none());
}

#[test]
fn list_with_items() {
    let v = tree("<ul><li>a</li><li>b</li></ul>");
    let l = find_first(&v, "list").unwrap();
    let items = l["children"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["role"], "listitem");
    assert_eq!(items[1]["name"], "b");
}

#[test]
fn img_with_alt_appears_as_img_role() {
    let v = tree("<img src='x.png' alt='Logo'>");
    let i = find_first(&v, "img").unwrap();
    assert_eq!(i["name"], "Logo");
}

#[test]
fn img_with_empty_alt_collapses_to_presentation_and_drops() {
    let v = tree("<img src='x.png' alt=''>");
    assert!(find_first(&v, "img").is_none());
    assert!(find_first(&v, "presentation").is_none());
}

#[test]
fn button_without_name_records_null() {
    let v = tree("<button></button>");
    let b = find_first(&v, "button").unwrap();
    assert_eq!(b["name"], Value::Null);
}

#[test]
fn form_control_carries_label_for_as_name() {
    let v = tree("<label for='q'>Search</label><input id='q'>");
    let inp = find_first(&v, "textbox").unwrap();
    assert_eq!(inp["name"], "Search");
}

#[test]
fn role_overrides_implicit() {
    let v = tree("<div role='button'>Click</div>");
    let b = find_first(&v, "button").unwrap();
    assert_eq!(b["name"], "Click");
}

#[test]
fn doctype_is_skipped_entirely() {
    let v = tree("<!doctype html><html><body><h1>Hi</h1></body></html>");
    let h = find_first(&v, "heading").unwrap();
    assert_eq!(h["name"], "Hi");
}
