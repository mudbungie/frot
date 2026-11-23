//! Every row is a Chrome 158 measurement (`Accessibility.getFullAXTree`,
//! `--headless=new`, `bl-89a6`).

use super::r;

#[test]
fn a_global_aria_attribute_overrules_presentation() {
    for attr in [
        "aria-atomic=true",
        "aria-braillelabel=b",
        "aria-busy=true",
        "aria-controls=z",
        "aria-current=page",
        "aria-describedby=z",
        "aria-description=d",
        "aria-details=z",
        "aria-flowto=z",
        "aria-keyshortcuts=k",
        "aria-label=U",
        "aria-label=''",
        "aria-labelledby=z",
        "aria-live=polite",
        "aria-owns=z",
        "aria-relevant=all",
        "aria-roledescription=r",
    ] {
        assert_eq!(
            r(&format!("<ul id=t role=none {attr}><li>i</ul>")),
            Some("list"),
            "{attr}"
        );
    }
    for attr in [
        "title=T",
        "aria-hidden=false",
        "aria-disabled=true",
        "aria-checked=true",
        "aria-expanded=true",
        "aria-haspopup=true",
        "aria-invalid=true",
        "aria-errormessage=z",
        "aria-dropeffect=copy",
        "aria-grabbed=true",
    ] {
        let html = format!("<ul id=t role=presentation {attr}><li>i</ul>");
        assert_eq!(r(&html), Some("presentation"), "{attr}");
    }
}

#[test]
fn the_implicit_role_returns_not_the_next_token() {
    assert_eq!(
        r("<h3 id=t role='presentation button' aria-label=y>p</h3>"),
        Some("heading")
    );
    assert_eq!(
        r("<div id=t role='none button' aria-label=x>n</div>"),
        Some("generic")
    );
    assert_eq!(r("<img id=t role=none alt=x>"), Some("none"));
}

#[test]
fn focus_overrules_presentation() {
    for html in [
        "<h3 id=t role=none tabindex=-1>h</h3>",
        "<h3 id=t role=none tabindex=' +2'>h</h3>",
        "<ul id=t role=none tabindex=0><li>i</ul>",
        "<button id=t role=none>b</button>",
        "<a id=t role=none href=#x>l</a>",
        "<input id=t role=none>",
        "<select id=t role=none><option>o</select>",
        "<textarea id=t role=none></textarea>",
        "<input id=t type=checkbox role=presentation>",
        "<details open><summary id=t role=none>s</summary>d</details>",
    ] {
        assert!(!matches!(r(html), Some("none" | "presentation")), "{html}");
    }
    for html in [
        "<h3 id=t role=none tabindex=foo>h</h3>",
        "<h3 id=t role=none tabindex=''>h</h3>",
        "<h3 id=t role=none tabindex=->h</h3>",
        "<button id=t role=none disabled>b</button>",
        "<fieldset disabled><button id=t role=none>b</button></fieldset>",
        "<a id=t role=none>l</a>",
        "<input id=t role=none type=HIDDEN>",
        "<details open><summary>s</summary><summary id=t role=none>s</summary></details>",
    ] {
        assert_eq!(r(html), Some("none"), "{html}");
    }
}

#[test]
fn a_list_items_role_follows_the_lists_role_attribute() {
    for list in [
        "<ul>",
        "<ol>",
        "<menu>",
        "<ul role=''>",
        "<ul role=list>",
        "<ul role=directory>",
    ] {
        assert_eq!(
            r(&format!("{list}<li id=t>i</li>")),
            Some("listitem"),
            "{list}"
        );
    }
    for list in [
        "<ul role=foo>",
        "<ol role=foo>",
        "<ul role=' '>",
        "<ul role=LIST>",
        "<ul role='foo list'>",
        "<ul role=region>",
        "<ul role=none aria-label=U>",
        "<div>",
    ] {
        assert_eq!(
            r(&format!("{list}<li id=t>i</li>")),
            Some("generic"),
            "{list}"
        );
    }
}
