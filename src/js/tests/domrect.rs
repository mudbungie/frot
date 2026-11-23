//! `DOMRect` and `DOMRectReadOnly` are constructable (`bl-e65f`). Every row
//! was read off Firefox 153.0esr (identity.md §3.19 rig): argument conversion,
//! the derived edges under negative sizes, NaN and infinities, `fromRect`'s
//! dictionary, and the receiver checks. Only the engine's own message for a
//! Symbol argument differs (identity.md §11, "Engine error messages").

use super::sess;

const S: &str = "function S(r){return ['x','y','width','height','top','right','bottom','left']\
    .map(function(k){return String(r[k])}).join()}\
    function t(f){try{return String(f())}catch(e){return e.name+': '+e.message}}";

#[test]
fn a_rect_is_built_and_read_as_the_binarys() {
    let s = sess("<html><body></body></html>");
    s.eval(S).unwrap();
    for (expr, want) in [
        ("S(new DOMRect(1,'2',undefined,null))", "1,2,0,0,2,1,2,1"),
        ("S(new DOMRect(10,10,-5,-3))", "10,10,-5,-3,7,10,10,5"),
        ("S(new DOMRect(NaN,1,2,3))", "NaN,1,2,3,1,NaN,4,NaN"),
        ("S(new DOMRect(1,1,NaN,-3))", "1,1,NaN,-3,-2,NaN,1,NaN"),
        (
            "S(new DOMRect(Infinity,-Infinity,-Infinity,Infinity))",
            "Infinity,-Infinity,-Infinity,Infinity,NaN,NaN,NaN,NaN",
        ),
        ("S(new DOMRect({},[],[5],'0x10'))", "NaN,0,5,16,0,NaN,16,NaN"),
        (
            "(function(){var r=new DOMRect(-0,0,0,0);return [Object.is(r.x,-0),Object.is(r.left,-0)]})()",
            "true,true",
        ),
        (
            "(function(){var r=new DOMRect();r.x=3;r.width='4';r.top=9;var top=r.top;\
             r.height=undefined;return [r.x,r.width,top,r.right,r.height]})()",
            "3,4,0,7,NaN",
        ),
        (
            "(function(){var r=new DOMRectReadOnly(1,2,3,4);r.x=9;return [r.x,r instanceof DOMRect,\
             JSON.stringify(r)]})()",
            "1,false,{\"x\":1,\"y\":2,\"width\":3,\"height\":4,\"top\":2,\"right\":4,\"bottom\":6,\"left\":1}",
        ),
        (
            "(function(){var log=[];var v=function(n){return {valueOf:function(){log.push(n);return 1}}};\
             new DOMRect(v('x'),v('y'),v('w'),v('h'));return log})()",
            "x,y,w,h",
        ),
        (
            "(function(){class R extends DOMRect{};var r=new R(1,2,3,4);return [r instanceof R,r.width,\
             Object.getOwnPropertyNames(r).length]})()",
            "true,3,0",
        ),
        (
            "[Object.getPrototypeOf(DOMRect)===DOMRectReadOnly,DOMRect.length,DOMRectReadOnly.length,\
             Object.getOwnPropertyNames(DOMRect).join(' ')]",
            "true,0,0,length name prototype fromRect",
        ),
        ("document.body.getBoundingClientRect() instanceof DOMRect", "true"),
    ] {
        assert_eq!(s.eval(expr).unwrap(), want, "{expr}");
    }
}

#[test]
fn from_rect_reads_a_dictionary_as_the_binary_does() {
    let s = sess("<html><body></body></html>");
    s.eval(S).unwrap();
    for (expr, want) in [
        ("S(DOMRect.fromRect({x:1,y:2,width:3}))", "1,2,3,0,2,4,2,1"),
        (
            "S(DOMRect.fromRect({x:'3',width:null,height:undefined}))",
            "3,0,0,0,0,3,0,3",
        ),
        (
            "(function(){var f=function(){};f.x=4;return S(DOMRect.fromRect(f))})()",
            "4,0,0,0,0,4,0,4",
        ),
        (
            "[DOMRectReadOnly.fromRect().constructor.name,DOMRect.fromRect(null).constructor.name,\
             DOMRect.fromRect===DOMRectReadOnly.fromRect,DOMRect.fromRect.length]",
            "DOMRectReadOnly,DOMRect,false,0",
        ),
        (
            "(function(){var log=[],o={};['height','width','x','y','z'].forEach(function(k){\
             Object.defineProperty(o,k,{get:function(){log.push(k);return 1}})});\
             DOMRect.fromRect(o);return log})()",
            "height,width,x,y",
        ),
        (
            "t(function(){return DOMRect.fromRect(5)})",
            "TypeError: DOMRect.fromRect: Argument 1 can't be converted to a dictionary.",
        ),
        (
            "t(function(){return DOMRectReadOnly.fromRect(true)})",
            "TypeError: DOMRectReadOnly.fromRect: Argument 1 can't be converted to a dictionary.",
        ),
    ] {
        assert_eq!(s.eval(expr).unwrap(), want, "{expr}");
    }
}

#[test]
fn a_member_on_the_wrong_receiver_says_which_interface() {
    let s = sess("<html><body></body></html>");
    s.eval(S).unwrap();
    let d = "Object.getOwnPropertyDescriptor";
    for (expr, want) in [
        (
            format!("t(function(){{return {d}(DOMRectReadOnly.prototype,'x').get.call({{}})}})"),
            "TypeError: 'get x' called on an object that does not implement interface DOMRectReadOnly.",
        ),
        (
            format!("t(function(){{return {d}(DOMRectReadOnly.prototype,'top').get.call({{}})}})"),
            "TypeError: 'get top' called on an object that does not implement interface DOMRectReadOnly.",
        ),
        (
            format!(
                "t(function(){{return {d}(DOMRect.prototype,'x').set.call(new DOMRectReadOnly(),1)}})"
            ),
            "TypeError: 'set x' called on an object that does not implement interface DOMRect.",
        ),
        (
            format!("t(function(){{return {d}(DOMRect.prototype,'y').get.call(new DOMRectReadOnly(0,5))}})"),
            "TypeError: 'get y' called on an object that does not implement interface DOMRect.",
        ),
        (
            "t(function(){return DOMRectReadOnly.prototype.toJSON.call({})})".to_string(),
            "TypeError: 'toJSON' called on an object that does not implement interface DOMRectReadOnly.",
        ),
    ] {
        assert_eq!(s.eval(&expr).unwrap(), want, "{expr}");
    }
}
