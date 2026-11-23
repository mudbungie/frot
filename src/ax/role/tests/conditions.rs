//! The conditional HTML-AAM mappings, each with its inverse.
//!
//! Every expectation here is a Chrome full-AX observation from the report in
//! `bl-a189` or the condition the HTML-AAM states for it.

use super::*;

/// Every `input` type, with the role Chrome exposes for it. `hidden` is the
/// only one that is not exposed at all.
const INPUT_TYPES: &[(&str, Option<&str>)] = &[
    ("hidden", None),
    ("button", Some("button")),
    ("image", Some("button")),
    ("reset", Some("button")),
    ("submit", Some("button")),
    ("checkbox", Some("checkbox")),
    ("radio", Some("radio")),
    ("range", Some("slider")),
    ("number", Some("spinbutton")),
    ("search", Some("searchbox")),
    ("text", Some("textbox")),
    ("email", Some("textbox")),
    ("tel", Some("textbox")),
    ("url", Some("textbox")),
    ("password", Some("textbox")),
    ("color", Some("textbox")),
    ("file", Some("textbox")),
    ("date", Some("textbox")),
    ("datetime-local", Some("textbox")),
    ("month", Some("textbox")),
    ("time", Some("textbox")),
    ("week", Some("textbox")),
    ("wibble", Some("textbox")),
];

#[test]
fn every_input_type_maps() {
    for (ty, want) in INPUT_TYPES {
        assert_eq!(r(&format!("<input id=t type='{ty}'>")), *want, "type={ty}");
    }
}

#[test]
fn input_type_is_ascii_case_insensitive() {
    assert_eq!(r("<input id=t type=CHECKBOX>"), Some("checkbox"));
    assert_eq!(r("<input id=t type=Hidden>"), None);
}

#[test]
fn input_without_type_is_a_textbox() {
    assert_eq!(r("<input id=t>"), Some("textbox"));
}

#[test]
fn a_list_attribute_makes_text_entry_a_combobox() {
    for ty in ["text", "email", "tel", "url", "search"] {
        let html = format!("<input id=t type='{ty}' list=d><datalist id=d></datalist>");
        assert_eq!(r(&html), Some("combobox"), "type={ty}");
    }
    // Non-text-entry types ignore `list` entirely.
    assert_eq!(r("<input id=t type=number list=d>"), Some("spinbutton"));
    assert_eq!(r("<input id=t type=checkbox list=d>"), Some("checkbox"));
}

#[test]
fn th_scope_decides_the_header_axis() {
    for (scope, want) in [
        ("row", "rowheader"),
        ("rowgroup", "rowheader"),
        ("ROW", "rowheader"),
        ("col", "columnheader"),
        ("colgroup", "columnheader"),
    ] {
        let html = format!("<table><tr><th id=t scope='{scope}'>H</th><th>H2</th></tr></table>");
        assert_eq!(r(&html), Some(want), "scope={scope}");
    }
}

#[test]
fn a_scopeless_th_opening_a_data_row_is_a_rowheader() {
    // Wikipedia's accessibility tables: the first cell of each body row.
    let html = "<table><tr><!--c--><th id=t>Name</th><td>Value</td></tr></table>";
    assert_eq!(r(html), Some("rowheader"));
}

#[test]
fn a_scopeless_th_in_an_all_header_row_is_a_columnheader() {
    let html = "<table><tr><th id=t>A</th><th>B</th></tr></table>";
    assert_eq!(r(html), Some("columnheader"));
}

#[test]
fn a_scopeless_th_reads_its_siblings() {
    // Chrome 158 (`bl-7f3a`): a full `<td>` (any child node) beside a header,
    // or at either end of the row or one in from it, makes it head its row.
    for (row, want) in [
        ("<td>V</td><th id=t>A</th>", "rowheader"),
        ("<th id=t>x</th><td></td>", "columnheader"),
        ("<th id=t>x</th><td> </td>", "rowheader"),
        ("<th id=t>x</th>", "columnheader"),
        ("<td></td><th id=t>h</th><td></td>", "columnheader"),
        ("<td></td><th id=t>h</th><th>g</th><td>d</td>", "rowheader"),
        (
            "<td>d</td><th>a</th><th id=t>h</th><th>g</th>",
            "columnheader",
        ),
        ("<td>d</td><th>a</th><th>h</th><th id=t>g</th>", "rowheader"),
        ("<th id=t>a</th><th>h</th><td>e</td><td></td>", "rowheader"),
        ("<th id=t>a</th><th>h</th><th>g</th>", "columnheader"),
    ] {
        let html = format!("<table><tr>{row}</tr></table>");
        assert_eq!(r(&html), Some(want), "{row}");
    }
}

#[test]
fn cells_follow_the_ancestor_tables_role() {
    assert_eq!(r("<table><tr><td id=t>d</td></tr></table>"), Some("cell"));
    for grid in ["grid", "treegrid"] {
        let html = format!("<table role='{grid}'><tr><td id=t>d</td></tr></table>");
        assert_eq!(r(&html), Some("gridcell"), "{grid}");
    }
    // The nearest table decides: a data table nested in a grid has cells, and
    // a table the author made presentational has none — it does not hand the
    // question outward.
    let nested = "<table role=grid><tr><td><table><tr><th>h</th><td id=t>d</td></tr></table>";
    assert_eq!(r(nested), Some("cell"));
    let through = "<table role=grid><tr><td><table role=none><tr><td id=t>d</td></tr></table>";
    assert_eq!(r(through), Some("generic"));
    // Headers keep their axis roles inside a grid.
    let html = "<table role=grid><tr><th id=t scope=row>H</th><td>d</td></tr></table>";
    assert_eq!(r(html), Some("rowheader"));
}

#[test]
fn a_section_is_a_region_only_when_named() {
    assert_eq!(r("<section id=t>x</section>"), Some("generic"));
    assert_eq!(
        r("<section id=t aria-label='Intro'>x</section>"),
        Some("region")
    );
    assert_eq!(r("<section id=t title='Intro'>x</section>"), Some("region"));
    assert_eq!(
        r("<h2 id=h>Intro</h2><section id=t aria-labelledby=h>x</section>"),
        Some("region")
    );
    // A name that computes to nothing is no name.
    assert_eq!(
        r("<section id=t aria-label='  '>x</section>"),
        Some("generic")
    );
    assert_eq!(
        r("<section id=t aria-labelledby=missing>x</section>"),
        Some("generic")
    );
}

/// `bl-2a3c`, Chrome 156: a `<form>` is a `form` named or not, wherever it
/// stands. (`bl-a189` had made it named-only, from a landmark count.)
#[test]
fn a_form_is_a_form_named_or_not() {
    for html in [
        "<form id=t><input></form>",
        "<form id=t></form>",
        "<form id=t aria-label='Site search'><input></form>",
        "<search><form id=t><input type=search></form></search>",
        "<nav><form id=t><input></form></nav>",
        "<form id=t role=bogus><input></form>",
    ] {
        assert_eq!(r(html), Some("form"), "{html}");
    }
    // An author role still replaces it.
    assert_eq!(r("<form id=t role=search><input></form>"), Some("search"));
    assert_eq!(r("<form id=t role=none><input></form>"), Some("none"));
}

#[test]
fn an_aside_is_complementary_at_top_level_and_named_inside_a_section() {
    assert_eq!(r("<aside id=t>x</aside>"), Some("complementary"));
    assert_eq!(
        r("<article><aside id=t>x</aside></article>"),
        Some("generic")
    );
    assert_eq!(
        r("<article><aside id=t aria-label='Notes'>x</aside></article>"),
        Some("complementary")
    );
}

#[test]
fn header_and_footer_are_page_landmarks_only_outside_a_sectioning_ancestor() {
    assert_eq!(r("<header id=t>x</header>"), Some("banner"));
    assert_eq!(r("<footer id=t>x</footer>"), Some("contentinfo"));
    assert_eq!(r("<div><header id=t>x</header></div>"), Some("banner"));
}

#[test]
fn a_sectioned_header_or_footer_is_section_scoped() {
    for tag in ["article", "aside", "main", "nav", "section"] {
        let html = format!("<{tag}><header id=t>x</header></{tag}>");
        assert_eq!(r(&html), Some("sectionheader"), "{tag}");
    }
    // Sectioning *roots* scope them too — Stanford's blockquote footer.
    for html in [
        "<blockquote><footer id=t>x</footer></blockquote>",
        "<details><footer id=t>x</footer></details>",
        "<dialog><footer id=t>x</footer></dialog>",
        "<figure><footer id=t>x</footer></figure>",
        "<fieldset><footer id=t>x</footer></fieldset>",
        "<table><tr><td><footer id=t>x</footer></td></tr></table>",
    ] {
        assert_eq!(r(html), Some("sectionfooter"), "{html}");
    }
}

#[test]
fn a_list_item_needs_no_list() {
    // Chrome 158 (`bl-d61c`): an `<li>` outside a list is a list item too.
    for html in [
        "<ul><li id=t>x</li></ul>",
        "<ol><li id=t>x</li></ol>",
        "<menu><li id=t>x</li></menu>",
        "<div><li id=t>x</li></div>",
        "<li id=t>x</li>",
        "<dir role=foo><li id=t>x</li></dir>",
    ] {
        assert_eq!(r(html), Some("listitem"), "{html}");
    }
}

#[test]
fn empty_alt_is_decorative_unless_something_else_names_it() {
    assert_eq!(r("<img id=t alt=''>"), Some("presentation"));
    assert_eq!(r("<img id=t alt='Logo'>"), Some("img"));
    assert_eq!(r("<img id=t>"), Some("img"));
    assert_eq!(r("<img id=t alt='' aria-label='Logo'>"), Some("img"));
    assert_eq!(r("<img id=t alt='' title='Logo'>"), Some("img"));
}

/// Every caption source has the role, empty or not: the table caption with
/// no line in it is left out by the tree (`ax/tree/lineless.rs`), not here.
#[test]
fn a_caption_is_a_caption() {
    for html in [
        "<table><caption id=t></caption><tr><th>a</th><td>b</td></tr></table>",
        "<table><caption id=t>words</caption><tr><th>a</th><td>b</td></tr></table>",
        "<figure><figcaption id=t></figcaption></figure>",
        "<div id=t role=caption></div>",
    ] {
        assert_eq!(r(html), Some("caption"), "{html}");
    }
}

/// Chrome 158 (`bl-dfc3`): a `title` attribute, even an empty one, lets a
/// landmark stand; an empty `aria-label` does not, and a `<tbody>` or an
/// `<img alt="">` wants a real name.
#[test]
fn an_empty_title_still_claims_a_landmark() {
    assert_eq!(r("<div id=t role=form title=''>x</div>"), Some("form"));
    assert_eq!(r("<div id=t role=region title=''>x</div>"), Some("region"));
    assert_eq!(r("<section id=t title=''>x</section>"), Some("region"));
    let nested = "<article><aside id=t title=''>x</aside></article>";
    assert_eq!(r(nested), Some("complementary"));
    assert_eq!(
        r("<div id=t role=form aria-label=''>x</div>"),
        Some("generic")
    );
    let tbody = "<table><tbody id=t title=''><tr><th>a</th><td>b</td></tr></table>";
    assert_eq!(r(tbody), Some("generic"));
    assert_eq!(r("<img id=t alt='' title=''>"), Some("presentation"));
}
