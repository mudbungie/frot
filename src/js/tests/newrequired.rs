//! A constructable interface called as a plain function (`bl-6404`). Measured
//! on Firefox 153.0esr (identity.md §3.20): each throws `TypeError: <Name>
//! constructor: 'new' is required` before it looks at an argument, and an
//! interface a page cannot construct throws `Illegal constructor.` either way.

use super::sess;

/// Every constructable interface frot publishes whose message was read off
/// the binary.
const CONSTRUCTABLE: &str = "Event CustomEvent FileReader ProgressEvent Blob Headers Response \
    Request XMLHttpRequest AbortController TextEncoder TextDecoder Text Comment DocumentFragment \
    MutationObserver IntersectionObserver ResizeObserver EventTarget URL URLSearchParams Worker \
    SharedWorker Notification FormData Image AudioContext OfflineAudioContext OscillatorNode \
    ImageData IDBVersionChangeEvent PerformanceObserver";

const ILLEGAL: &str = "Node Element Storage Navigator NodeList CryptoKey AbortSignal Location \
    CSSStyleDeclaration";

fn calls(names: &str, how: &str) -> String {
    sess("<html><body></body></html>")
        .eval(&format!(
            "'{names}'.split(/\\s+/).map(function(n){{try{{{how};return n+' did not throw'}}\
             catch(e){{return e.name+': '+e.message}}}}).join('\\n')"
        ))
        .unwrap()
}

#[test]
fn a_constructable_interface_called_without_new_says_so() {
    let want: Vec<String> = CONSTRUCTABLE
        .split_whitespace()
        .map(|n| format!("TypeError: {n} constructor: 'new' is required"))
        .collect();
    assert_eq!(calls(CONSTRUCTABLE, "globalThis[n]()"), want.join("\n"));
    // With arguments too: the check comes before any argument is read.
    assert_eq!(
        calls(CONSTRUCTABLE, "globalThis[n]('x',{},1)"),
        want.join("\n")
    );
    // And through `call` on an object that already has the prototype.
    assert_eq!(
        calls("Event", "Event.call(Object.create(Event.prototype),'x')"),
        "TypeError: Event constructor: 'new' is required"
    );
}

#[test]
fn an_interface_a_page_cannot_construct_is_illegal_either_way() {
    let n = ILLEGAL.split_whitespace().count();
    let want = vec!["TypeError: Illegal constructor."; n].join("\n");
    assert_eq!(calls(ILLEGAL, "globalThis[n]()"), want);
    assert_eq!(calls(ILLEGAL, "new globalThis[n]()"), want);
}

/// `new` still constructs, subtypes still initialize as events, and the
/// signals a controller and the statics hand out are real.
#[test]
fn construction_itself_is_unchanged() {
    let s = sess("<html><body></body></html>");
    for (expr, want) in [
        ("new Event('x').type+new CustomEvent('y',{detail:3}).detail", "x3"),
        ("(function(){var e=new ProgressEvent('p',{loaded:4});return e.type+e.loaded+\
          (e instanceof Event)})()", "p4true"),
        ("(function(){var e=new IDBVersionChangeEvent('v',{oldVersion:2});return e.type+\
          e.oldVersion+(e instanceof Event)})()", "v2true"),
        ("String(Reflect.construct(Event,['x']))", "[object Event]"),
        ("(function(){var c=new AbortController(),s=c.signal;c.abort();return [s instanceof \
          AbortSignal,s.aborted,AbortSignal.abort().aborted,AbortSignal.timeout(5).aborted,\
          AbortSignal.any([s]).aborted,String(s.onabort)]})()", "true,true,true,false,true,null"),
        ("new URL('https://a.example/p?q=1').searchParams.get('q')+new URLSearchParams('a=2').get('a')", "12"),
        ("String(new EventTarget())+typeof new MutationObserver(function(){}).observe", "[object EventTarget]function"),
    ] {
        assert_eq!(s.eval(expr).unwrap(), want, "{expr}");
    }
}
