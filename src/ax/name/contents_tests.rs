//! Name from contents: the §2F recursion over descendant text alternatives.
//!
//! Each case states the Chrome name observed in `bl-3d2e` or the rule the
//! accname algorithm gives for it.

use super::*;

fn name(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), None)
}

fn name_css(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    let s = crate::css::compute(&d);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), Some(&s))
}

/// The Wikipedia logo link: two images, no DOM text. Chrome names the link
/// "Wikipedia The Free Encyclopedia"; frot used to emit null.
#[test]
fn wikipedia_logo_link_names_from_two_image_alts() {
    let html = "<a href='/wiki/Main_Page'><img src=logo.png alt='Wikipedia'>\
                <img src=word.png alt='The Free Encyclopedia'></a>";
    assert_eq!(
        name(html, "a"),
        Some("Wikipedia The Free Encyclopedia".into())
    );
}

/// The W3C search button: named from the graphic it contains.
#[test]
fn button_names_from_a_child_svg_title() {
    let html = "<button><svg><title>Submit Search</title><path/></svg></button>";
    assert_eq!(name(html, "button"), Some("Submit Search".into()));
}

#[test]
fn an_svg_without_a_title_contributes_nothing() {
    assert_eq!(name("<button><svg><path/></svg></button>", "button"), None);
}

#[test]
fn a_linked_article_image_names_its_link() {
    assert_eq!(
        name("<a href='/a'><img alt='Braille display'></a>", "a"),
        Some("Braille display".into())
    );
}

#[test]
fn text_and_images_mix_in_tree_order() {
    assert_eq!(
        name("<a href='/x'>Read<img alt='the manual'>now</a>", "a"),
        Some("Read the manual now".into())
    );
}

#[test]
fn inline_text_is_concatenated_without_invented_spaces() {
    // Browsers name this "Helloworld": only alternatives are fenced.
    assert_eq!(
        name("<button>Hello<span>world</span></button>", "button"),
        Some("Helloworld".into())
    );
}

#[test]
fn a_decorative_image_contributes_nothing() {
    assert_eq!(
        name("<a href='/x'>Home<img alt=''></a>", "a"),
        Some("Home".into())
    );
    assert_eq!(name("<a href='/x'><img alt=''></a>", "a"), None);
}

#[test]
fn an_image_with_no_alt_contributes_nothing() {
    assert_eq!(name("<a href='/x'><img src=p.png></a>", "a"), None);
}

#[test]
fn a_descendant_aria_label_replaces_its_subtree() {
    assert_eq!(
        name(
            "<button><span aria-label='Close'>x</span></button>",
            "button"
        ),
        Some("Close".into())
    );
}

#[test]
fn a_descendant_aria_labelledby_is_followed() {
    let html = "<span id=lbl>Delete row</span>\
                <button><span aria-labelledby=lbl>x</span></button>";
    assert_eq!(name(html, "button"), Some("Delete row".into()));
}

#[test]
fn a_labelledby_that_resolves_to_nothing_falls_back_to_contents() {
    assert_eq!(
        name(
            "<button><span aria-labelledby=gone>Fallback</span></button>",
            "button"
        ),
        Some("Fallback".into())
    );
}

#[test]
fn a_self_referential_labelledby_terminates() {
    assert_eq!(
        name(
            "<button><span id=s aria-labelledby=s>Loop</span></button>",
            "button"
        ),
        Some("Loop".into())
    );
}

#[test]
fn a_mutual_labelledby_cycle_terminates() {
    let html = "<button><span id=a aria-labelledby=b>A</span>\
                <span id=b aria-labelledby=a>B</span></button>";
    // `a` follows the reference to `b`, which cannot come back to `a`; `b` is
    // then spent, so the whole name is what `b` said.
    assert_eq!(name(html, "button"), Some("B".into()));
}

#[test]
fn an_aria_hidden_descendant_is_not_read() {
    assert_eq!(
        name(
            "<button>Save<span aria-hidden=true>(beta)</span></button>",
            "button"
        ),
        Some("Save".into())
    );
    assert_eq!(
        name("<button>Save<span inert>(beta)</span></button>", "button"),
        Some("Save".into())
    );
    // An excluded image cannot smuggle its alt in either.
    assert_eq!(
        name(
            "<button>Save<img aria-hidden=true alt='beta'></button>",
            "button"
        ),
        Some("Save".into())
    );
}

#[test]
fn css_hidden_descendants_are_not_read() {
    let html = "<style>.a{display:none}.b{visibility:hidden}</style>\
                <button>Save<span class=a>draft</span><span class=b>copy</span></button>";
    assert_eq!(name_css(html, "button"), Some("Save".into()));
    // Without `--css` there is no styles table, so the text is all there is.
    assert_eq!(name(html, "button"), Some("Savedraftcopy".into()));
}

#[test]
fn generated_content_is_read() {
    let html = "<style>button::before{content:'New '}button::after{content:' item'}</style>\
                <button>list</button>";
    assert_eq!(name_css(html, "button"), Some("New list item".into()));
}

#[test]
fn embedded_controls_speak_their_values() {
    assert_eq!(
        name("<h2>Rows: <input type=number value='12'></h2>", "h2"),
        Some("Rows: 12".into())
    );
    assert_eq!(
        name("<h2>Note: <textarea>typed</textarea></h2>", "h2"),
        Some("Note: typed".into())
    );
    assert_eq!(
        name("<h2>Sort: <input type=image alt='by date'></h2>", "h2"),
        Some("Sort: by date".into())
    );
    assert_eq!(name("<h2>Empty: <input></h2>", "h2"), Some("Empty:".into()));
}

#[test]
fn an_embedded_select_speaks_its_selected_option() {
    let html = "<h2>Size: <select><option>S</option><option selected>M</option></select></h2>";
    assert_eq!(name(html, "h2"), Some("Size: M".into()));
    // No explicit selection: the UA selects the first option.
    let html = "<h2>Size: <select><option>S</option><option>M</option></select></h2>";
    assert_eq!(name(html, "h2"), Some("Size: S".into()));
    // Nothing to select.
    assert_eq!(
        name("<h2>Size: <select></select></h2>", "h2"),
        Some("Size:".into())
    );
}

#[test]
fn comments_and_doctypes_contribute_nothing() {
    assert_eq!(
        name("<!doctype html><button>Go<!--hint--></button>", "button"),
        Some("Go".into())
    );
}

#[test]
fn a_nested_name_from_contents_role_is_read_as_its_own_name() {
    let html = "<table><tr><td><a href='/x'><img alt='Edit'></a></td></tr></table>";
    assert_eq!(name(html, "td"), Some("Edit".into()));
}

#[test]
fn container_roles_still_do_not_name_from_their_subtree() {
    // The pre-existing restriction: only name-from-contents roles descend.
    assert_eq!(name("<ul><li>one</li></ul>", "ul"), None);
    assert_eq!(name("<nav><a href='/x'>Home</a></nav>", "nav"), None);
    assert_eq!(name("<p><img alt='X'></p>", "p"), None);
}

#[test]
fn a_title_attribute_is_still_the_last_resort() {
    // Contents win over `title`…
    assert_eq!(
        name("<button title='Tip'>Press</button>", "button"),
        Some("Press".into())
    );
    // …and `title` still answers when the contents are empty.
    assert_eq!(
        name("<button title='Tip'><img alt=''></button>", "button"),
        Some("Tip".into())
    );
}

#[test]
fn a_label_reads_its_own_image_alternatives() {
    let html = "<label for=q><img alt='Search'></label><input id=q>";
    assert_eq!(name(html, "input"), Some("Search".into()));
}

#[test]
fn a_label_does_not_read_back_the_control_it_labels() {
    // Otherwise the control would name itself after its own value.
    let html = "<label>Nickname <input value='Bob'></label>";
    assert_eq!(name(html, "input"), Some("Nickname".into()));
}

#[test]
fn a_labelledby_target_reads_alternatives_too() {
    let html = "<span id=l><img alt='Zoom'></span><button aria-labelledby=l></button>";
    assert_eq!(name(html, "button"), Some("Zoom".into()));
}

#[test]
fn a_legend_reads_alternatives_too() {
    let html = "<fieldset><legend><img alt='Colours'></legend></fieldset>";
    assert_eq!(name(html, "fieldset"), Some("Colours".into()));
}

#[test]
fn label_text_is_css_aware() {
    let html = "<style>.x{display:none}</style><label for=q>Search<span class=x>(beta)</span></label><input id=q>";
    assert_eq!(name_css(html, "input"), Some("Search".into()));
}

#[test]
fn a_closed_details_body_is_not_read_into_the_name() {
    // The UA conceals everything but the first `<summary>`, text nodes
    // included (`bl-74a6`), so the name is the summary alone.
    let html = "<h2><details><summary>Sum</summary>Body text</details></h2>";
    assert_eq!(name_css(html, "h2"), Some("Sum".into()));
    assert_eq!(name(html, "h2"), Some("SumBody text".into()));
}
