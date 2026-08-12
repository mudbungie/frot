//! The two ES iterator helpers `prelude/iterator.js` composes (bl-5249).
//!
//! The defect they answer is invisible from inside a run: the engine's own
//! `Iterator.prototype.find`/`.filter` leak every value their predicate
//! rejects, the run *completes*, and only engine teardown aborts the process —
//! `tests/binary.rs` pins that at the process boundary. What is pinned here is
//! that the replacements are the helpers a page expects: values found, sources
//! closed, laziness kept, and the built-ins' own shape (non-constructible,
//! native-reading, un-hijackable) preserved.

use super::sess;

#[test]
fn find_and_filter_walk_a_query_and_chain_the_engines_helpers() {
    let s = sess("<body><p>a</p><p>b</p><p>c</p></body>");
    let eval = |src: &str| s.eval(src).unwrap();
    assert_eq!(
        eval(
            "document.querySelectorAll('p').values().find(e => e.textContent === 'c').textContent"
        ),
        "c"
    );
    assert_eq!(
        eval("String(document.querySelectorAll('p').values().find(e => false))"),
        "undefined"
    );
    assert_eq!(
        eval(
            "document.querySelectorAll('p').values().filter(e => e.textContent !== 'b')\
             .map(e => e.textContent).toArray().join('')"
        ),
        "ac"
    );
    // The index every predicate gets is the source's, not the survivors'.
    assert_eq!(
        eval("[9, 8, 7].values().filter((v, i) => i > 0).toArray().join('')"),
        "87"
    );
}

#[test]
fn both_helpers_close_the_source_they_stop_reading() {
    let s = sess("<body></body>");
    // A source that records its own close, so every path below is observable.
    let eval = |body: &str| {
        s.eval(&format!(
            "(() => {{ var log = '';\
             var src = [1, 2, 3].values();\
             src.return = () => {{ log += 'R'; return {{ done: true }} }};\
             {body}\
             return log }})()"
        ))
        .unwrap()
    };
    // Lazy: `filter` pulls only what its consumer asks for, and the early exit
    // closes the source underneath it.
    assert_eq!(
        eval("[...src.filter(v => { log += v; return true }).take(1)];"),
        "1R"
    );
    // A hit closes the walk; the values past it are never pulled.
    assert_eq!(eval("src.find(v => { log += v; return v === 2 });"), "12R");
    // Closed at suspended start too: `return()` before the first `next()`.
    assert_eq!(eval("src.filter(v => true).return();"), "R");
    // A non-callable predicate closes before it throws (ES2026 27.1.3.3.4-.5),
    // and it throws rather than walking on.
    assert_eq!(
        eval("try { src.find(1) } catch (e) { log += e.name }"),
        "RTypeError"
    );
    assert_eq!(
        eval("try { src.filter(null) } catch (e) { log += e.name }"),
        "RTypeError"
    );
}

#[test]
fn the_replacements_keep_the_shape_of_the_built_ins_they_replace() {
    let s = sess("<body></body>");
    let eval = |src: &str| s.eval(src).unwrap();
    // Not constructors, exactly like the built-in methods (and every other
    // helper): no `prototype`, and `new` throws.
    assert_eq!(
        eval("String(Iterator.prototype.find.prototype)"),
        "undefined"
    );
    assert_eq!(
        eval("(() => { try { new Iterator.prototype.filter(v => v) } catch (e) { return e.name } })()"),
        "TypeError"
    );
    // The helpers are generic over iterator objects, so an own `some`/`flatMap`
    // on the iterator itself is lawful and must not reach the composition.
    assert_eq!(
        eval(
            "(() => { var found = [1].values(); found.some = null;\
             var kept = [2].values(); kept.flatMap = null;\
             return Iterator.prototype.find.call(found, v => true) + \
             [...Iterator.prototype.filter.call(kept, v => true)] })()"
        ),
        "12"
    );
    // Composed out of helpers captured before the swap, so a page that
    // overrides `some`/`flatMap` afterwards redirects only its own calls.
    assert_eq!(
        eval(
            "(() => { Iterator.prototype.some = () => { throw new Error('hijacked') };\
             Iterator.prototype.flatMap = () => { throw new Error('hijacked') };\
             return [1, 2].values().find(v => v === 2) + \
             [...[3, 4].values().filter(v => v === 4)] })()"
        ),
        "24"
    );
    // And they read as native code, like every other prelude-implemented API.
    assert!(eval("Iterator.prototype.find.toString()").contains("[native code]"));
}
