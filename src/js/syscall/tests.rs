//! Every syscall closure is driven here through raw `__frot_*` calls, so the
//! Rust binding layer reaches 100% coverage without leaning on the JS prelude
//! (which `llvm-cov` cannot see — that is the golden suite's job, js.md §3).

use crate::dom::Document;
use crate::fetch::FetchSession;
use crate::js::{Env, EvalError, Log, Session, StyleSource};

fn sess(html: &str) -> Session {
    let env = Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
    };
    Session::new(
        Document::parse(html),
        StyleSource::Bare,
        env,
        &FetchSession::new(Vec::new()),
    )
}

#[test]
fn reads_expose_kind_tag_attr_text_and_links() {
    let s = sess("<!DOCTYPE html><html><body><p id='x' class='a b'>hi</p><!--c--></body></html>");
    // roots are the doctype then <html>; kind labels every node variant.
    assert_eq!(s.eval("__frot_kind(__frot_roots()[0])").unwrap(), "doctype");
    // a root has no parent.
    assert_eq!(
        s.eval("String(__frot_parent(__frot_roots()[0]))").unwrap(),
        "undefined"
    );
    s.eval("globalThis.p = __frot_query_doc('p')[0]").unwrap();
    assert_eq!(s.eval("__frot_kind(p)").unwrap(), "element");
    assert_eq!(s.eval("__frot_tag(p)").unwrap(), "p");
    assert_eq!(s.eval("__frot_attr(p, 'id')").unwrap(), "x");
    assert_eq!(
        s.eval("String(__frot_attr(p, 'missing'))").unwrap(),
        "undefined"
    );
    // attrs enumerates an element's [name, value] pairs (the faithful-clone
    // primitive); on a non-element it is the empty list, not an error.
    assert_eq!(
        s.eval("JSON.stringify(__frot_attrs(p))").unwrap(),
        "[[\"id\",\"x\"],[\"class\",\"a b\"]]"
    );
    assert_eq!(s.eval("__frot_text(p)").unwrap(), "hi");
    assert_eq!(s.eval("__frot_tag(__frot_parent(p))").unwrap(), "body");
    // p's only child is a text node (the `text` kind arm).
    assert_eq!(
        s.eval("__frot_kind(__frot_children(p)[0])").unwrap(),
        "text"
    );
    // body's children are the element then the comment (the `comment` arm).
    s.eval("globalThis.body = __frot_query_doc('body')[0]")
        .unwrap();
    assert_eq!(
        s.eval("JSON.stringify(__frot_children(body).map(__frot_kind))")
            .unwrap(),
        "[\"element\",\"comment\"]"
    );
    // tag / attr / attrs on a non-element node fall to the empty arms.
    assert_eq!(
        s.eval("String(__frot_tag(__frot_children(body)[1]))")
            .unwrap(),
        "undefined"
    );
    assert_eq!(
        s.eval("String(__frot_attr(__frot_children(body)[1], 'x'))")
            .unwrap(),
        "undefined"
    );
    assert_eq!(
        s.eval("__frot_attrs(__frot_children(body)[1]).length")
            .unwrap(),
        "0"
    );
}

#[test]
fn mutations_build_relink_and_detach_nodes() {
    let s = sess("<html><body></body></html>");
    s.eval("globalThis.body = __frot_query_doc('body')[0]")
        .unwrap();
    // create_element lowercases; set_attr + insert_child splice it in.
    s.eval("globalThis.d = __frot_create_element('DIV')")
        .unwrap();
    s.eval("__frot_set_attr(d, 'ID', 'main')").unwrap();
    s.eval("__frot_insert_child(body, d, 0)").unwrap();
    assert_eq!(s.eval("__frot_tag(d)").unwrap(), "div");
    assert_eq!(s.eval("__frot_attr(d, 'id')").unwrap(), "main");
    // create_text + set_text, then splice under d.
    s.eval("globalThis.t = __frot_create_text('old')").unwrap();
    s.eval("__frot_set_text(t, 'new')").unwrap();
    s.eval("__frot_insert_child(d, t, 0)").unwrap();
    assert_eq!(s.eval("__frot_text(d)").unwrap(), "new");
    // remove_attr, then detach unlinks the text (subtree unreachable).
    s.eval("__frot_remove_attr(d, 'id')").unwrap();
    assert_eq!(s.eval("String(__frot_attr(d, 'id'))").unwrap(), "undefined");
    s.eval("__frot_detach(t)").unwrap();
    assert_eq!(s.eval("__frot_text(d)").unwrap(), "");
    // fragment parsing returns detached top-level ids to splice in.
    s.eval("globalThis.f = __frot_fragment('<span>a</span><span>b</span>')")
        .unwrap();
    assert_eq!(s.eval("f.length").unwrap(), "2");
    s.eval("f.forEach((id, i) => __frot_insert_child(d, id, i))")
        .unwrap();
    assert_eq!(s.eval("__frot_text(d)").unwrap(), "ab");
    // Mutations are visible on the one shared document.
    assert_eq!(s.document().find_by_tag("span").len(), 2);
}

#[test]
fn query_scopes_document_and_element_and_rejects_unsupported() {
    let s = sess("<html><body><div id='r'><p>a</p></div><p>b</p></body></html>");
    assert_eq!(s.eval("__frot_query_doc('p').length").unwrap(), "2");
    s.eval("globalThis.r = __frot_query_doc('#r')[0]").unwrap();
    assert_eq!(s.eval("__frot_query(r, 'p').length").unwrap(), "1");
    // Both query entrypoints throw a SyntaxError naming the selector.
    match s.eval("__frot_query(r, 'p:hover')").unwrap_err() {
        EvalError::Exception(m) => assert!(m.contains("unsupported selector: p:hover"), "{m}"),
        e => panic!("expected exception, got {e:?}"),
    }
    assert!(matches!(
        s.eval("__frot_query_doc(':bogus')").unwrap_err(),
        EvalError::Exception(_)
    ));
}

#[test]
fn console_syscall_appends_level_and_text() {
    let s = sess("<html></html>");
    s.eval("__frot_console('warn', 'hi there')").unwrap();
    let logs = s.console();
    assert_eq!(logs.len(), 1);
    assert_eq!(
        logs[0],
        Log {
            level: "warn".into(),
            text: "hi there".into()
        }
    );
}
