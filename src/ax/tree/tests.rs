use super::*;

fn tree(html: &str) -> Value {
    ax_tree(&Document::parse(html), None, None)
}

fn tree_css(html: &str) -> Value {
    let doc = Document::parse(html);
    let s = crate::css::compute(&doc);
    ax_tree(&doc, Some(&s), None)
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

fn count_role(v: &Value, role: &str) -> usize {
    let mut n = 0;
    if let Value::Array(arr) = v {
        for item in arr {
            if item["role"] == role {
                n += 1;
            }
            n += count_role(&item["children"], role);
        }
    }
    n
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
    // strong is not a name-from-contents role, so it carries no name.
    assert_eq!(s["name"], Value::Null);
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
    // listitem is a container role: not named from its contents.
    assert_eq!(items[1]["role"], "listitem");
    assert_eq!(items[1]["name"], Value::Null);
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

#[test]
fn css_display_none_drops_node_and_subtree() {
    let v = tree_css("<main style='display:none'><h1>Hidden</h1></main><h2>Shown</h2>");
    let h = find_first(&v, "heading").expect("the visible heading");
    assert_eq!(h["name"], "Shown");
    assert!(find_first(&v, "main").is_none());
}

#[test]
fn css_visibility_hidden_promotes_visible_children() {
    let v = tree_css(
        "<section style='visibility:hidden'>\
         <h1 style='visibility:visible'>Hi</h1></section>",
    );
    let h = find_first(&v, "heading").expect("heading surfaces");
    assert_eq!(h["name"], "Hi");
    assert!(find_first(&v, "region").is_none());
}

#[test]
fn css_generated_content_joins_accessible_name() {
    let v = tree_css("<style>button::before{content:'★ '}</style><button>Star</button>");
    let b = find_first(&v, "button").expect("button node");
    assert_eq!(b["name"], "★ Star");
}

#[test]
fn layout_table_collapses_scaffolding_surfacing_content() {
    let v = tree("<table><tr><td><a href='/x'>Home</a></td><td>text</td></tr></table>");
    assert!(find_first(&v, "table").is_none());
    assert!(find_first(&v, "rowgroup").is_none());
    assert!(find_first(&v, "row").is_none());
    assert!(find_first(&v, "cell").is_none());
    assert_eq!(find_first(&v, "link").unwrap()["name"], "Home");
}

#[test]
fn data_table_with_th_preserves_structure() {
    let v = tree("<table><tr><th>H</th></tr><tr><td>D</td></tr></table>");
    assert!(find_first(&v, "table").is_some());
    assert!(find_first(&v, "row").is_some());
    assert!(find_first(&v, "cell").is_some());
    assert!(find_first(&v, "columnheader").is_some());
}

#[test]
fn data_table_with_caption_preserves_structure() {
    let v = tree("<table><caption>Cap</caption><tr><td>D</td></tr></table>");
    assert!(find_first(&v, "table").is_some());
    assert!(find_first(&v, "cell").is_some());
}

#[test]
fn table_with_summary_attr_preserves_structure() {
    let v = tree("<table summary='s'><tr><td>D</td></tr></table>");
    assert!(find_first(&v, "table").is_some());
}

#[test]
fn table_with_role_attr_preserves_structure() {
    let v = tree("<table role='table'><tr><td>D</td></tr></table>");
    assert!(find_first(&v, "table").is_some());
    assert!(find_first(&v, "cell").is_some());
}

#[test]
fn table_with_aria_label_preserves_structure() {
    let v = tree("<table aria-label='Prices'><tr><td>D</td></tr></table>");
    assert!(find_first(&v, "table").is_some());
}

#[test]
fn table_with_aria_labelledby_preserves_structure() {
    let v = tree("<table aria-labelledby='h'><tr><td>D</td></tr></table>");
    assert!(find_first(&v, "table").is_some());
}

#[test]
fn nested_data_table_inside_layout_table() {
    let v = tree(
        "<table><tr><td>\
         <table><tr><th>H</th></tr><tr><td>D</td></tr></table>\
         </td></tr></table>",
    );
    // Outer (layout) collapses; only the inner data table remains, with its
    // header cell and a single data cell (the outer <td> collapsed away).
    assert_eq!(count_role(&v, "table"), 1);
    assert_eq!(count_role(&v, "cell"), 1);
    assert!(find_first(&v, "columnheader").is_some());
}
