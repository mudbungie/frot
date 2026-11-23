//! What a descendant says in a name, and what an `<input>` is named — each a
//! Chrome 158 measurement (`bl-d94b`); `names.html` holds the same cases.

use super::*;

fn h3(inner: &str) -> Option<String> {
    let html = format!("<h3>a{inner}c</h3><p id=pd2>referenced</p>");
    let d = Document::parse(&html);
    let s = crate::css::compute(&d);
    accessible_name(&d, d.find_by_tag("h3")[0], Some(&s))
}

fn input(attrs: &str) -> Option<String> {
    let d = Document::parse(&format!("<label>{attrs}</label>"));
    accessible_name(&d, d.find_by_tag("input")[0], None)
}

#[test]
fn a_container_role_gives_only_its_own_name() {
    for (inner, want) in [
        ("<span role=combobox>X</span>", "ac"),
        ("<span role=group>X</span>", "ac"),
        ("<span role=img>X</span>", "ac"),
        ("<span role=listbox>X</span>", "a c"),
        ("<span role=group aria-label=GL>X</span>", "a GL c"),
        (
            "<span role=img aria-labelledby=pd2>X</span>",
            "a referenced c",
        ),
        ("<fieldset><legend>lg</legend>fs</fieldset>", "a lg c"),
        ("<fieldset>nolegend</fieldset>", "a c"),
        ("<figure>fg</figure>", "a c"),
        ("<output>out</output>", "a c"),
        ("<object>obj</object>", "ac"),
        ("<map><area alt=ar href=x></map>", "ac"),
        (
            "<details open><summary>sm</summary>db</details>",
            "a sm db c",
        ),
        ("<span style='display:inline-block'></span>", "ac"),
        ("<span style='display:block'></span>", "a c"),
        ("<button></button>", "a c"),
    ] {
        assert_eq!(h3(inner).as_deref(), Some(want), "{inner}");
    }
    // The node a reference names reads its own contents, whatever its role.
    let html = "<button aria-labelledby=g>x</button><span id=g role=group>GG</span>";
    let d = Document::parse(html);
    assert_eq!(
        accessible_name(&d, d.find_by_tag("button")[0], None).as_deref(),
        Some("GG")
    );
}

#[test]
fn a_range_widget_says_its_value() {
    for (inner, want) in [
        ("<span role=slider>X</span>", "a 50 c"),
        ("<span role=scrollbar>X</span>", "a 50 c"),
        ("<span role=spinbutton>X</span>", "a 0 c"),
        ("<span role=meter>X</span>", "a 0 c"),
        (
            "<span role=slider aria-valuemin=10 aria-valuemax=20>X</span>",
            "a 15 c",
        ),
        ("<span role=slider aria-valuenow=7>X</span>", "a 7 c"),
        (
            "<span role=slider aria-valuenow=7 aria-valuetext=seven>X</span>",
            "a seven c",
        ),
        (
            "<span role=spinbutton aria-valuenow=3 aria-valuetext=' '>X</span>",
            "a 3 c",
        ),
        ("<input type=range>", "a 50 c"),
        ("<input type=range value=30>", "a 30 c"),
        ("<input type=range min=10 max=20>", "a 15 c"),
        ("<input type=number value=4>", "a 4 c"),
        ("<br>", "a c"),
    ] {
        assert_eq!(h3(inner).as_deref(), Some(want), "{inner}");
    }
}

#[test]
fn an_input_says_its_default_label() {
    for (inner, want) in [
        ("<input type=submit>", "a Submit c"),
        ("<input type=reset>", "a Reset c"),
        ("<input type=image>", "a Submit c"),
        ("<input type=image alt=IA>", "a IA c"),
        ("<input type=image alt='' value=IV>", "a IV c"),
        ("<input type=button>", "a c"),
        ("<input type=password value=pw>", "a \u{2022}\u{2022} c"),
        ("<input type=password>", "a c"),
    ] {
        assert_eq!(h3(inner).as_deref(), Some(want), "{inner}");
    }
}

#[test]
fn an_input_button_is_named_by_default() {
    for (attrs, want) in [
        ("<input type=submit>", Some("Submit")),
        ("<input type=SUBMIT>", Some("Submit")),
        ("<input type=reset>", Some("Reset")),
        ("<input type=submit title=ST>", Some("Submit")),
        ("<input type=submit value=''>", None),
        ("<input type=reset value=R title=RT>", Some("R")),
        ("Wrapped <input type=submit value=V>", Some("Wrapped")),
        ("<input type=button>", None),
        ("<input type=button title=BT>", Some("BT")),
        ("<input type=image>", Some("Submit")),
        ("<input type=image title=IT>", Some("IT")),
        ("<input type=image alt=''>", Some("Submit")),
        ("<input type=image value=IV>", Some("IV")),
        ("<input type=image alt=IA value=IV>", Some("IA")),
    ] {
        assert_eq!(input(attrs).as_deref(), want, "{attrs}");
    }
}
