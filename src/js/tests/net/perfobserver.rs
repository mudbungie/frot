//! `PerformanceObserver` (`bl-b129`): the shape and errors read off Firefox
//! 153.0esr (identity.md §3.20), and the one entry type frot delivers —
//! `resource`, for requests that really went out.

use super::session_at;
use mockito::Server;

fn caught(s: &crate::js::Session, expr: &str) -> String {
    s.eval(&format!(
        "(function(){{try{{return String({expr})}}catch(e){{return e.name+': '+e.message}}}})()"
    ))
    .unwrap()
}

#[test]
fn the_surface_and_its_errors_are_as_measured() {
    let s = session_at("https://example.com/");
    let f = "new PerformanceObserver(function(){})";
    let at = "TypeError: PerformanceObserver.observe: Can't call observe";
    let moved = "InvalidModificationError: An attempt was made to modify the type of the \
                 underlying objec";
    for (expr, want) in [
        ("Object.getOwnPropertyNames(PerformanceObserver.prototype)", "observe,disconnect,takeRecords,constructor"),
        ("Object.getOwnPropertyNames(PerformanceObserverEntryList.prototype)",
         "getEntries,getEntriesByType,getEntriesByName,constructor"),
        ("[PerformanceObserver.length,String(new PerformanceObserver(function(){})),\
          Object.getOwnPropertyNames(new PerformanceObserver(function(){})).length]",
         "1,[object PerformanceObserver],0"),
        ("PerformanceObserver.supportedEntryTypes",
         "event,first-input,largest-contentful-paint,mark,measure,navigation,paint,resource"),
        ("PerformanceObserver.supportedEntryTypes===PerformanceObserver.supportedEntryTypes", "false"),
        ("PerformanceObserver(function(){})", "TypeError: PerformanceObserver constructor: 'new' is required"),
        ("new PerformanceObserver()",
         "TypeError: PerformanceObserver constructor: At least 1 argument required, but only 0 passed"),
        ("new PerformanceObserver(5)", "TypeError: PerformanceObserver constructor: Argument 1 is not an object."),
        ("new PerformanceObserver({})", "TypeError: PerformanceObserver constructor: Argument 1 is not callable."),
        ("new PerformanceObserverEntryList()", "TypeError: Illegal constructor."),
        (&format!("{f}.observe()"), &format!("{at} without `type` or `entryTypes`")),
        (&format!("{f}.observe({{}})"), &format!("{at} without `type` or `entryTypes`")),
        (&format!("{f}.observe({{entryTypes:['resource'],type:'resource'}})"),
         &format!("{at} with both `type` and `entryTypes`")),
        (&format!("{f}.observe({{entryTypes:['bogus']}})"), "undefined"),
        (&format!("{f}.observe({{type:'bogus'}})"), "undefined"),
        (&format!("{f}.observe({{entryTypes:[]}})"), "undefined"),
        (&format!("(function(){{var o={f};o.observe({{entryTypes:['resource']}});\
                   return o.observe({{type:'navigation'}})}})()"), moved),
        (&format!("(function(){{var o={f};o.observe({{type:'navigation'}});\
                   return o.observe({{entryTypes:['resource']}})}})()"), moved),
        (&format!("(function(){{var o={f};o.observe({{entryTypes:['resource','navigation']}});\
                   o.observe({{entryTypes:['resource']}});var r=o.takeRecords();\
                   return Array.isArray(r)+','+r.length+','+o.disconnect()}})()"), "true,0,undefined"),
    ] {
        assert_eq!(caught(&s, expr), want.replace("\n                 ", ""), "{expr}");
    }
}

/// A request that went out reaches every observer of `resource` as one task:
/// `callback(list, observer)` with the observer as `this`. Entries from before
/// `observe` are for `{type, buffered: true}` alone; `navigation` never comes.
#[test]
fn resource_entries_are_delivered_and_nothing_else_is() {
    let mut server = Server::new();
    let _a = server.mock("GET", "/a").with_body("x").create();
    let _b = server.mock("GET", "/b").with_body("x").create();
    let s = session_at(&server.url());
    s.eval(
        "var log=[];function mk(tag){var o=new PerformanceObserver(function(list,obs){\
         log.push(tag+':'+list.getEntries().map(function(e){return e.name.slice(-2)}).join('+')+':'+\
         (obs===o)+(this===o)+String(list)+list.getEntriesByType('resource').length+\
         list.getEntriesByType('navigation').length+list.getEntriesByName(list.getEntries()[0].name)\
         .length)});return o}\
         fetch('/a');",
    )
    .unwrap();
    s.eval(
        "var early=mk('early');early.observe({entryTypes:['resource','navigation']});\
         var nav=mk('nav');nav.observe({type:'navigation',buffered:true});\
         var buf=mk('buf');buf.observe({type:'resource',buffered:true});\
         var gone=mk('gone');gone.observe({type:'resource'});gone.disconnect();\
         var bad=new PerformanceObserver(function(){throw new Error('po-throw')});\
         bad.observe({type:'resource'});\
         fetch('/b');",
    )
    .unwrap();
    assert_eq!(s.eval("log.length").unwrap(), "0", "delivery is a task");
    assert_eq!(
        s.eval("while(__frot_next_timer()>=0);log.join()").unwrap(),
        "buf:/a+/b:truetrue[object PerformanceObserverEntryList]201,\
         early:/b:truetrue[object PerformanceObserverEntryList]101"
    );
    // A disconnected observer hears nothing, and one that starts late hears
    // only what follows.
    s.eval(
        "early.disconnect();buf.disconnect();bad.disconnect();log=[];\
         var late=mk('late');late.observe({type:'resource'});fetch('/a?2')",
    )
    .unwrap();
    assert_eq!(
        s.eval("while(__frot_next_timer()>=0);log.join()").unwrap(),
        "late:?2:truetrue[object PerformanceObserverEntryList]101"
    );
    s.eval("late.disconnect();log=[];fetch('/b?3')").unwrap();
    assert_eq!(
        s.eval("while(__frot_next_timer()>=0);log.length").unwrap(),
        "0"
    );
}
