//! `FormData` (`bl-6eb9`): the entry list as Firefox 153.0esr shapes it, and
//! the `multipart/form-data` text a `fetch`/XHR POST of one puts on the wire
//! (identity.md §3.20). The server here echoes what it received, so the
//! assertion is on the real request, boundary and all.

use mockito::Server;

use super::session_at;

fn page() -> crate::js::Session {
    session_at("https://example.com/")
}

fn caught(s: &crate::js::Session, expr: &str) -> String {
    s.eval(&format!(
        "(function(){{try{{return String({expr})}}catch(e){{return e.name+': '+e.message}}}})()"
    ))
    .unwrap()
}

#[test]
fn form_data_is_firefox_shaped() {
    let s = page();
    assert_eq!(
        caught(&s, "Object.getOwnPropertyNames(FormData.prototype)"),
        "append,delete,get,getAll,has,set,entries,keys,values,forEach,constructor"
    );
    assert_eq!(
        caught(
            &s,
            "['append','delete','get','getAll','has','set','entries','keys','values','forEach']\
             .map(function(k){return FormData.prototype[k].length+FormData.prototype[k].name})"
        ),
        "2append,1delete,1get,1getAll,1has,2set,0entries,0keys,0values,1forEach"
    );
    assert_eq!(
        caught(
            &s,
            "[String(new FormData()), Object.getOwnPropertyNames(new FormData()).length, \
             FormData.prototype[Symbol.iterator]===FormData.prototype.entries, FormData.length, \
             String(new FormData(undefined)), typeof __frot_multipart]"
        ),
        "[object FormData],0,true,0,[object FormData],undefined"
    );
}

#[test]
fn the_entry_list_behaves_as_measured() {
    let s = page();
    s.eval(
        "var fd=new FormData(); fd.append('a','1'); fd.append('b',2); fd.append('a','x'); \
         fd.append('u',undefined); fd.append('n',null);",
    )
    .unwrap();
    assert_eq!(
        caught(&s, "JSON.stringify(Array.from(fd))"),
        r#"[["a","1"],["b","2"],["a","x"],["u","undefined"],["n","null"]]"#
    );
    assert_eq!(
        caught(
            &s,
            "[fd.get('a'), fd.get('zz'), fd.get(5), JSON.stringify(fd.getAll('a')), fd.has('b'), \
             fd.has('zz'), typeof fd.get('b'), Array.from(fd.keys()), Array.from(fd.values())]\
             .map(String).join(' ')"
        ),
        r#"1 null null ["1","x"] true false string a,b,a,u,n 1,2,x,undefined,null"#
    );
    // set replaces the first of that name in place and drops the rest; a new
    // name is appended. delete removes every entry of the name.
    assert_eq!(
        caught(
            &s,
            "(fd.set('a','only'), fd.set('z','new'), fd['delete']('u'), fd['delete']('nope'), \
             JSON.stringify(Array.from(fd.entries())))"
        ),
        r#"[["a","only"],["b","2"],["n","null"],["z","new"]]"#
    );
    assert_eq!(
        caught(
            &s,
            "(function(){var o=[],t={};fd.forEach(function(v,k,f){o.push(k+'='+v+(f===fd)+\
             (this===t))},t);return o.join('&')})()"
        ),
        "a=onlytruetrue&b=2truetrue&n=nulltruetrue&z=newtruetrue"
    );
    for (call, want) in [
        (
            "fd.append()",
            "FormData.append: 0 is not a valid argument count for any overload.",
        ),
        (
            "fd.append('a')",
            "FormData.append: 1 is not a valid argument count for any overload.",
        ),
        (
            "fd.set('a')",
            "FormData.set: 1 is not a valid argument count for any overload.",
        ),
        (
            "fd.get()",
            "FormData.get: At least 1 argument required, but only 0 passed",
        ),
        (
            "fd.getAll()",
            "FormData.getAll: At least 1 argument required, but only 0 passed",
        ),
        (
            "fd.has()",
            "FormData.has: At least 1 argument required, but only 0 passed",
        ),
        (
            "fd['delete']()",
            "FormData.delete: At least 1 argument required, but only 0 passed",
        ),
        (
            "fd.forEach()",
            "FormData.forEach: Argument 1 is not an object.",
        ),
        (
            "fd.forEach(5)",
            "FormData.forEach: Argument 1 is not an object.",
        ),
        (
            "new FormData(5)",
            "FormData constructor: Argument 1 is not an object.",
        ),
        (
            "new FormData(null)",
            "FormData constructor: Argument 1 is not an object.",
        ),
        (
            "new FormData({})",
            "FormData constructor: Argument 1 does not implement interface HTMLFormElement.",
        ),
        (
            "new FormData(document.createElement('div'))",
            "FormData constructor: Argument 1 does not implement interface HTMLFormElement.",
        ),
        (
            "FormData.prototype.get.call({}, 'a')",
            "'get' called on an object that does not implement interface FormData.",
        ),
        (
            "FormData.prototype.keys.call({})",
            "'keys' called on an object that does not implement interface FormData.",
        ),
        // Declared short of Firefox: frot does not read a form's controls.
        (
            "new FormData(document.createElement('form'))",
            "frot: FormData from a form element is not supported",
        ),
    ] {
        assert_eq!(caught(&s, call), format!("TypeError: {want}"), "{call}");
    }
}

/// POST a FormData through `send` (JS that issues the request and stores the
/// echoed `content-type` + body in `out`); return them.
fn posted(fill: &str, send: &str) -> (String, String) {
    let mut server = Server::new();
    let m = server
        .mock("POST", "/v")
        .with_body_from_request(|req| {
            let ctype = req.header("content-type")[0].to_str().unwrap().to_string();
            format!("{ctype}\n{}", req.utf8_lossy_body().unwrap()).into_bytes()
        })
        .expect(1)
        .create();
    let s = session_at(&server.url());
    s.eval(&format!("var out='', fd=new FormData(); {fill}; {send}"))
        .unwrap();
    m.assert();
    let out = s.eval("out").unwrap();
    let (ctype, body) = out.split_once('\n').unwrap();
    (ctype.to_string(), body.to_string())
}

const FETCH: &str = "fetch('/v',{method:'POST',body:fd}).then(function(r){return r.text()})\
                     .then(function(t){out=t})";

#[test]
fn a_posted_form_data_is_firefox_multipart_text() {
    let (ctype, body) = posted(
        "fd.append('a','1'); fd.append('v','a\\nb\\rc\\r\\nd'); fd.append('n\\n\"q','é')",
        FETCH,
    );
    let b = ctype
        .strip_prefix("multipart/form-data; boundary=")
        .expect("multipart content type");
    // `----geckoformboundary` + two unpadded 64-bit hex numbers.
    let tail = b
        .strip_prefix("----geckoformboundary")
        .expect("gecko boundary");
    assert!((2..=32).contains(&tail.len()), "{tail}");
    assert!(tail
        .bytes()
        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
    // Line breaks become CRLF everywhere; a name's CR, LF and quote are escaped.
    assert_eq!(
        body,
        format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n1\r\n\
             --{b}\r\nContent-Disposition: form-data; name=\"v\"\r\n\r\na\r\nb\r\nc\r\nd\r\n\
             --{b}\r\nContent-Disposition: form-data; name=\"n%0D%0A%22q\"\r\n\r\né\r\n\
             --{b}--\r\n"
        )
    );
    // An empty FormData is the closing delimiter alone; boundaries are random.
    let (ctype2, empty) = posted("", FETCH);
    let b2 = ctype2
        .strip_prefix("multipart/form-data; boundary=")
        .unwrap();
    assert_eq!(empty, format!("--{b2}--\r\n"));
    assert_ne!(b, b2);
}

#[test]
fn xhr_and_an_explicit_content_type_carry_a_form_data_too() {
    let (ctype, body) = posted(
        "fd.append('k','v')",
        "var x=new XMLHttpRequest(); x.open('POST','/v'); \
         x.onload=function(){out=x.responseText}; x.send(fd)",
    );
    let b = ctype
        .strip_prefix("multipart/form-data; boundary=")
        .unwrap();
    assert_eq!(
        body,
        format!("--{b}\r\nContent-Disposition: form-data; name=\"k\"\r\n\r\nv\r\n--{b}--\r\n")
    );
    // A Content-Type the script set itself wins, exactly as for any other body.
    let (ctype, body) = posted(
        "fd.append('k','v')",
        "fetch('/v',{method:'POST',headers:{'Content-Type':'x/y'},body:fd})\
         .then(function(r){return r.text()}).then(function(t){out=t})",
    );
    assert_eq!(ctype, "x/y");
    assert!(body.contains("name=\"k\""));
}

/// A `Blob` value is kept as that Blob, and a FormData holding one is bytes the
/// request path cannot carry: the POST fails rather than send `[object Blob]`.
#[test]
fn a_form_data_holding_a_blob_is_not_sent_as_text() {
    let mut server = Server::new();
    let m = server.mock("POST", "/v").expect(0).create();
    let s = session_at(&server.url());
    s.eval(
        "var out='', fd=new FormData(); fd.append('f', new Blob(['hi'])); \
         fd.set('g', new Blob(['yo']), 'name.bin'); \
         fetch('/v',{method:'POST',body:fd}).catch(function(e){out=String(e)})",
    )
    .unwrap();
    assert!(s.eval("out").unwrap().contains("only text request bodies"));
    m.assert();
    assert_eq!(
        caught(
            &s,
            "[fd.get('f') instanceof Blob, fd.get('f').size, fd.get('g').size]"
        ),
        "true,2,2"
    );
    // The filename form is only for a Blob value.
    assert_eq!(
        caught(&s, "fd.append('a','str','name')"),
        "TypeError: FormData.append: Argument 2 is not an object."
    );
}
