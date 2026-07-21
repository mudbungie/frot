use super::*;

fn doc(html: &str) -> Document {
    Document::parse(html)
}

fn first_element(d: &Document, tag: &str) -> NodeId {
    *d.find_by_tag(tag).first().unwrap()
}

fn name(html: &str, tag: &str) -> Option<String> {
    let d = doc(html);
    accessible_name(&d, first_element(&d, tag), None)
}

fn name_css(html: &str, tag: &str) -> Option<String> {
    let d = doc(html);
    let s = crate::css::compute(&d);
    accessible_name(&d, first_element(&d, tag), Some(&s))
}

#[test]
fn text_node_has_no_name() {
    let d = doc("<p>hi</p>");
    // grab the text node id under <p>
    let p = first_element(&d, "p");
    let text_child = d.node(p).children[0];
    assert_eq!(accessible_name(&d, text_child, None), None);
}

#[test]
fn aria_label_wins() {
    assert_eq!(
        name("<button aria-label='Close'>X</button>", "button"),
        Some("Close".into())
    );
}

#[test]
fn empty_aria_label_falls_through() {
    assert_eq!(
        name("<button aria-label=''>Send</button>", "button"),
        Some("Send".into())
    );
}

#[test]
fn aria_labelledby_joins_referenced_text() {
    let html =
        "<span id='a'>Hello</span><span id='b'>world</span><button aria-labelledby='a b'></button>";
    assert_eq!(name(html, "button"), Some("Hello world".into()));
}

#[test]
fn aria_labelledby_skips_missing_ids() {
    let html = "<span id='a'>Hello</span><button aria-labelledby='a missing'></button>";
    assert_eq!(name(html, "button"), Some("Hello".into()));
}

#[test]
fn aria_labelledby_all_missing_falls_through() {
    let html = "<button aria-labelledby='nope'>Click</button>";
    assert_eq!(name(html, "button"), Some("Click".into()));
}

#[test]
fn button_text_content_is_name() {
    assert_eq!(
        name("<button>Submit</button>", "button"),
        Some("Submit".into())
    );
}

#[test]
fn anchor_text_content_is_name() {
    assert_eq!(name("<a href='/'>Home</a>", "a"), Some("Home".into()));
}

#[test]
fn heading_text_content_is_name() {
    assert_eq!(name("<h1>Title</h1>", "h1"), Some("Title".into()));
}

#[test]
fn img_alt_is_name() {
    assert_eq!(
        name("<img src='x.png' alt='Logo'>", "img"),
        Some("Logo".into())
    );
}

#[test]
fn img_empty_alt_is_some_empty_then_falls_to_title() {
    // empty alt produces an empty native name; algorithm falls through to title
    assert_eq!(
        name("<img src='x.png' alt='' title='Tip'>", "img"),
        Some("Tip".into())
    );
}

#[test]
fn img_without_alt_or_title_has_no_name() {
    assert_eq!(name("<img src='x.png'>", "img"), None);
}

#[test]
fn area_alt_is_name() {
    let html = "<map><area shape='rect' alt='Zone' href='/z'></map>";
    assert_eq!(name(html, "area"), Some("Zone".into()));
}

#[test]
fn input_button_value_is_name() {
    let html = "<input type='button' value='Go'>";
    assert_eq!(name(html, "input"), Some("Go".into()));
}

#[test]
fn input_submit_value_is_name() {
    let html = "<input type='submit' value='Send'>";
    assert_eq!(name(html, "input"), Some("Send".into()));
}

#[test]
fn input_reset_value_is_name() {
    let html = "<input type='reset' value='Reset'>";
    assert_eq!(name(html, "input"), Some("Reset".into()));
}

#[test]
fn input_image_alt_is_name() {
    let html = "<input type='image' alt='Pixel'>";
    assert_eq!(name(html, "input"), Some("Pixel".into()));
}

#[test]
fn input_text_label_for_is_name() {
    let html = "<label for='q'>Search</label><input id='q' type='text'>";
    assert_eq!(name(html, "input"), Some("Search".into()));
}

#[test]
fn input_text_wrapping_label_is_name() {
    let html = "<label>Name <input type='text'></label>";
    assert_eq!(name(html, "input"), Some("Name".into()));
}

#[test]
fn textarea_label_for_is_name() {
    let html = "<label for='b'>Bio</label><textarea id='b'></textarea>";
    assert_eq!(name(html, "textarea"), Some("Bio".into()));
}

#[test]
fn select_label_for_is_name() {
    let html = "<label for='c'>Country</label><select id='c'><option>US</option></select>";
    assert_eq!(name(html, "select"), Some("Country".into()));
}

#[test]
fn control_with_no_label_falls_through_to_title_when_present() {
    let html = "<input type='text' title='Hint'>";
    assert_eq!(name(html, "input"), Some("Hint".into()));
}

#[test]
fn control_without_label_or_title_has_no_name() {
    let html = "<input type='text'>";
    assert_eq!(name(html, "input"), None);
}

#[test]
fn fieldset_legend_is_name() {
    let html = "<fieldset><legend>Profile</legend></fieldset>";
    assert_eq!(name(html, "fieldset"), Some("Profile".into()));
}

#[test]
fn fieldset_without_legend_falls_through_to_title() {
    let html = "<fieldset title='Misc'></fieldset>";
    assert_eq!(name(html, "fieldset"), Some("Misc".into()));
}

#[test]
fn title_is_last_resort() {
    let html = "<div title='Tip'></div>";
    assert_eq!(name(html, "div"), Some("Tip".into()));
}

#[test]
fn empty_title_does_not_match() {
    let html = "<div title=''></div>";
    assert_eq!(name(html, "div"), None);
}

#[test]
fn name_normalizes_whitespace() {
    let html = "<button>  Submit\n  request  </button>";
    assert_eq!(name(html, "button"), Some("Submit request".into()));
}

#[test]
fn empty_button_has_no_name() {
    let html = "<button></button>";
    assert_eq!(name(html, "button"), None);
}

#[test]
fn label_for_mismatch_falls_through() {
    let html = "<label for='other'>Search</label><input id='q'>";
    assert_eq!(name(html, "input"), None);
}

#[test]
fn css_generated_content_included_in_name() {
    assert_eq!(
        name_css(
            "<style>h1::before{content:'§ '}</style><h1>Title</h1>",
            "h1"
        ),
        Some("§ Title".into())
    );
}

#[test]
fn css_display_none_subtree_excluded_from_name() {
    assert_eq!(
        name_css("<h1>Vis<span style='display:none'>Gone</span></h1>", "h1"),
        Some("Vis".into())
    );
}

// --- name-from-contents role gating (bl-a864) -------------------------------
// Container roles must NOT be named from their subtree text; only roles in the
// WAI-ARIA "Name From: contents" set take the subtree-text fallback.

#[test]
fn paragraph_is_not_named_from_contents() {
    // role=paragraph is a container: no native name despite having text.
    assert_eq!(name("<p>Body copy text</p>", "p"), None);
}

#[test]
fn list_is_not_named_from_contents() {
    // role=list is a container: subtree text is not its name.
    assert_eq!(name("<ul><li>One</li><li>Two</li></ul>", "ul"), None);
}

#[test]
fn listitem_is_not_named_from_contents() {
    assert_eq!(name("<ul><li>One</li></ul>", "li"), None);
}

#[test]
fn table_is_not_named_from_contents() {
    let html = "<table><tr><td>R1C1</td></tr></table>";
    assert_eq!(name(html, "table"), None);
}

#[test]
fn roleless_element_is_not_named_from_contents() {
    // <br> has no ARIA role at all: the guard's None arm → no native name.
    assert_eq!(name("<br>Stray text after break", "br"), None);
}

#[test]
fn explicit_role_button_is_named_from_contents() {
    // Gating is role-based, not tag-based: role=button on a span still names.
    assert_eq!(
        name("<span role='button'>Go</span>", "span"),
        Some("Go".into())
    );
}

#[test]
fn explicit_role_list_suppresses_content_name() {
    assert_eq!(name("<span role='list'>All the items</span>", "span"), None);
}

#[test]
fn container_falls_through_to_title() {
    // A container with no content name still uses title as last resort.
    assert_eq!(
        name("<ul title='Menu'><li>Home</li></ul>", "ul"),
        Some("Menu".into())
    );
}
