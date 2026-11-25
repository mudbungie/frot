//! `display` values as Chrome 158 computes and lays them out (`bl-e3a0`).

use super::Display;
use crate::css::compute;
use crate::dom::Document;

fn display(html: &str, tag: &str) -> Display {
    let doc = Document::parse(html);
    let id = doc.find_by_tag(tag)[0];
    compute(&doc).display(id)
}

#[test]
fn every_value_chrome_accepts_takes_the_nearest_box() {
    let table = [
        ("none", Display::None),
        ("block", Display::Block),
        ("flow-root", Display::Block),
        ("grid", Display::Grid),
        ("table", Display::Table),
        ("table block", Display::Table),
        ("table-caption", Display::Block),
        ("table-column", Display::TableColumn),
        ("table-column-group", Display::TableColumn),
        ("-webkit-box", Display::Block),
        ("block flow", Display::Block),
        ("inline", Display::Inline),
        ("ruby", Display::Inline),
        ("ruby-text", Display::Inline),
        ("math", Display::Inline),
        ("initial", Display::Inline),
        ("unset", Display::Inline),
        ("inline-block", Display::InlineBlock),
        ("inline-grid", Display::InlineGrid),
        ("inline grid", Display::InlineGrid),
        ("inline-table", Display::InlineTable),
        ("table inline", Display::InlineTable),
        ("-webkit-inline-box", Display::InlineBlock),
        ("inline flow-root", Display::InlineBlock),
        ("list-item", Display::ListItem),
        ("flex", Display::Flex),
        ("block flex", Display::Flex),
        ("inline-flex", Display::InlineFlex),
        ("inline flex", Display::InlineFlex),
        ("contents", Display::Contents),
        ("table-cell", Display::TableCell),
        ("table-row", Display::TableRow),
        ("table-row-group", Display::TableRow),
        ("table-header-group", Display::TableRow),
        ("table-footer-group", Display::TableRow),
        // Keywords in any order (`bl-9dec`).
        ("flex inline", Display::InlineFlex),
        ("list-item inline", Display::Inline),
        ("flow-root inline", Display::InlineBlock),
        ("list-item block", Display::ListItem),
        ("list-item flow-root", Display::ListItem),
        ("flow list-item block", Display::ListItem),
        ("inline flow list-item", Display::Inline),
        ("grid block", Display::Grid),
        // Case-insensitive and trimmed.
        ("  INLINE-Block ", Display::InlineBlock),
    ];
    for (value, want) in table {
        assert_eq!(Display::parse(value), Some(want), "{value}");
    }
}

#[test]
fn a_value_chrome_rejects_is_no_declaration() {
    for value in [
        "run-in",
        "bogus",
        "inline-list-item",
        "masonry",
        "inline inline",
        "flex grid",
        "flex list-item",
        "revert",
        "",
    ] {
        assert_eq!(Display::parse(value), None, "{value}");
    }
    // The tag's own display stands.
    assert_eq!(
        display("<span style='display:bogus'>x</span>", "span"),
        Display::Inline
    );
    assert_eq!(
        display("<div style='display:run-in'>x</div>", "div"),
        Display::Block
    );
    // And so does the valid declaration it would have beaten.
    let html = "<style>i{display:inline-block;display:bogus}</style><i>x</i>";
    assert_eq!(display(html, "i"), Display::InlineBlock);
    let html = "<style>i{display:flex}</style><i style='display:bogus'>x</i>";
    assert_eq!(display(html, "i"), Display::Flex);
}

#[test]
fn revert_asks_for_the_user_agents_display() {
    assert!(Display::reverts(" REVERT "));
    assert!(!Display::reverts("revert-layer"));
    let html = "<style>li,b{display:none}</style>\
                <li style='display:revert'>x</li><b style='display:revert'>y</b>";
    assert_eq!(display(html, "li"), Display::ListItem);
    assert_eq!(display(html, "b"), Display::Inline);
    // `[hidden]` is the user agent's too.
    let html = "<style>p{display:block}</style><p hidden style='display:revert'>x</p>";
    assert_eq!(display(html, "p"), Display::None);
    // A pseudo-element's is `inline`, and its content is generated again.
    let html = "<style>p::before{content:'B';display:none}\
                p::before{display:revert}</style><p>x</p>";
    let doc = Document::parse(html);
    let styles = compute(&doc);
    let p = doc.find_by_tag("p")[0];
    assert_eq!(styles.before(p), Some("B"));
    assert_eq!(styles.before_display(p), Display::Inline);
}

#[test]
fn a_pseudo_element_drops_an_invalid_display_too() {
    let html = "<style>p::after{content:'A';display:none;display:bogus}\
                i::after{content:'A';display:block;display:run-in}</style><p>x</p><i>y</i>";
    let doc = Document::parse(html);
    let styles = compute(&doc);
    assert_eq!(styles.after(doc.find_by_tag("p")[0]), None);
    let i = doc.find_by_tag("i")[0];
    assert_eq!(styles.after_display(i), Display::Block);
}

#[test]
fn table_parts_stack_and_are_on_no_line() {
    for part in [Display::TableCell, Display::TableRow] {
        assert!(part.table_part() && part.stacks() && !part.block_level());
        assert_eq!(part.blockified(true), Display::Block);
        assert_eq!(part.blockified(false), part);
    }
    assert!(Display::Block.stacks() && !Display::Block.table_part());
    for other in [Display::Contents, Display::Inline, Display::InlineBlock] {
        assert!(!other.stacks() && !other.table_part() && !other.block_level());
    }
    // `contents` has no box to take out of flow.
    assert_eq!(Display::Contents.blockified(true), Display::Contents);
}

// Measured in Chrome 158 `getComputedStyle` and `innerText` (`bl-9dec`).
#[test]
fn inherit_takes_the_parents_display() {
    let block = "<div><p style='display:inherit'>x</p></div>";
    assert_eq!(display(block, "p"), Display::Block);
    let inline = "<span><p style='display:inherit'>y</p></span>";
    assert_eq!(display(inline, "p"), Display::Inline);
}

#[test]
fn a_flex_or_grid_item_is_blockified() {
    for container in ["flex", "inline-flex", "grid", "inline-grid"] {
        let html = format!(
            "<div style='display:{container}'><span>a</span><b style='display:inline-flex'>b</b>\
             <i style='display:inline-grid'>i</i><q style='display:contents'>q</q></div>"
        );
        assert_eq!(display(&html, "span"), Display::Block, "{container}");
        assert_eq!(display(&html, "b"), Display::Flex, "{container}");
        assert_eq!(display(&html, "i"), Display::Grid, "{container}");
        assert_eq!(display(&html, "q"), Display::Contents, "{container}");
    }
    let float = "<span style='display:inline-grid;float:left'>a</span>";
    assert_eq!(display(float, "span"), Display::Grid);
}

#[test]
fn a_closed_dialog_and_a_silent_audio_are_not_rendered() {
    assert_eq!(display("<dialog>d</dialog>", "dialog"), Display::None);
    assert_eq!(display("<dialog open>d</dialog>", "dialog"), Display::Block);
    let shown = "<dialog style='display:block'>d</dialog>";
    assert_eq!(display(shown, "dialog"), Display::Block);
    assert_eq!(display("<audio></audio>", "audio"), Display::None);
    let forced = "<audio style='display:block'></audio>";
    assert_eq!(display(forced, "audio"), Display::None);
    assert_eq!(
        display("<audio controls></audio>", "audio"),
        Display::Inline
    );
}

#[test]
fn a_table_column_paints_none_of_its_content() {
    let html = "<div>u1<span style='display:table-column'><b>u2</b>t</span>u3</div>";
    let doc = Document::parse(html);
    let styles = compute(&doc);
    let span = doc.find_by_tag("span")[0];
    assert_eq!(styles.display(span), Display::TableColumn);
    assert!(styles.display_none(doc.find_by_tag("b")[0]));
    let text = crate::views::text::text(&doc, Some(&styles));
    assert_eq!(text, "u1\nu3");
}

#[test]
fn a_popover_no_script_has_shown_is_not_rendered() {
    assert_eq!(display("<div popover>p</div>", "div"), Display::None);
    let open = "<dialog popover open>d</dialog>";
    assert_eq!(display(open, "dialog"), Display::Block);
    let forced = "<div popover style='display:flex'>p</div>";
    assert_eq!(display(forced, "div"), Display::Flex);
    let mut doc = Document::parse("<div popover>p</div><dialog>d</dialog>");
    let (div, dialog) = (doc.find_by_tag("div")[0], doc.find_by_tag("dialog")[0]);
    doc.set_popover(div, true);
    doc.set_popover(dialog, true);
    let styles = compute(&doc);
    assert_eq!(styles.display(div), Display::Block);
    assert_eq!(styles.display(dialog), Display::Block);
}
