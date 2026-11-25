//! Which image an `<img>` requests (`bl-0e4a`), against Firefox 153.0esr on
//! the rig (js.md §7 "Images"): the `<picture>`/`srcset`/`sizes` pick is what
//! `currentSrc` names and what the request is for, the picked density divides
//! the natural size, and a mutation that moves the pick starts a new request.

use super::imageload::run;

/// A whole 10×5 PNG.
const PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAoAAAAFCAIAAADzBuo/AAAADElEQVR4nGNgGLQAAACbAAEgYWrrAAAAAElFTkSuQmCC";

/// `watch(img, label)`: log each event with the URL and natural size.
const WATCH: &str = "function watch(i,k){['load','error'].forEach(function(t){i.addEventListener(t,\
    function(){L(k,t,i.currentSrc.replace(location.origin,'').slice(0,10),i.naturalWidth,i.naturalHeight)})})}";

/// Measured: a 101×51 image at 2x reports 51×26, at 300w for a 100px size
/// 34×17 — the size over the density, rounded half up.
#[test]
fn the_density_divides_the_natural_size() {
    let (out, report) = run(&format!(
        "{WATCH};[['x2','{PNG} 2x',''],['w','{PNG} 40w','10px'],['x05','{PNG} 0.5x','']]\
         .forEach(function(p){{var i=new Image();watch(i,p[0]);i.sizes=p[2];i.srcset=p[1]}})"
    ));
    assert_eq!(report.errors, 0);
    assert_eq!(
        out,
        "x2:load:data:image:5:3 w:load:data:image:3:1 x05:load:data:image:20:10"
    );
}

/// A parsed `<picture>` requests its first fitting `<source>`.
#[test]
fn a_parsed_picture_requests_its_fitting_source() {
    let (doc, report) = super::drive(
        "<body><picture><source type=image/jxl srcset=/jxl><source media='(min-width: 2000px)' \
         srcset=/big><source type=image/webp srcset='/webp 1x, /webp2 2x'><img src=/fb></picture>\
         <img srcset='/w400 400w, /w800 800w' sizes=50vw><picture><source srcset=/only><img></picture>\
         <script>var log=[];document.querySelectorAll('img').forEach(function(i){i.onerror=function(){\
         log.push(i.currentSrc.replace(location.origin,''))}});onload=function(){\
         document.body.setAttribute('data-out',log.join())}</script></body>",
    );
    assert_eq!(report.errors, 0);
    let body = doc.find_by_tag("body")[0];
    let crate::dom::NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    assert_eq!(el.attr("data-out"), Some("/webp,/w800,/only"));
}

/// Built by script, as a framework builds it: the `<img>` entering a
/// `<picture>` behind a `<source>` (or a `<source>` entering before it)
/// re-picks, and only the last request ends (measured: one request, for the
/// source's URL, in both orders).
#[test]
fn a_picture_built_by_script_requests_the_source() {
    let (out, _) = run(&format!(
        "{WATCH};function mk(k,order){{var p=document.createElement('picture'),s=document.createElement('source'),\
         i=document.createElement('img');watch(i,k);s.srcset='/s'+k;\
         if(order){{p.appendChild(s);i.src='/fb';p.appendChild(i)}}else{{p.appendChild(i);i.src='/fb';p.insertBefore(s,i)}}\
         document.body.appendChild(p);return [p,s,i]}}mk('a',1);mk('b',0);\
         var c=mk('c',1);c[1].srcset='/sc2';var d=mk('d',1);d[1].remove();\
         var e=mk('e',1);e[1].setAttribute('media','print');e[1].setAttribute('class','x');\
         var f=mk('f',1);f[0].innerHTML='<source srcset=/inner><img src=/fb2>';watch(f[0].lastChild,'f2');\
         var g=mk('g',1);document.body.appendChild(g[2]);\
         var h=mk('h',1);h[1].removeAttribute('srcset');h[1].removeAttribute('srcset');\
         var n=mk('n',1);var fr=document.createDocumentFragment();fr.appendChild(document.createElement('source'));\
         n[0].insertBefore(fr,n[1]);n[0].appendChild(document.createTextNode('t'))"
    ));
    assert_eq!(
        out,
        "a:error:/sa:0:0 b:error:/sb:0:0 c:error:/sc2:0:0 d:error:/fb:0:0 e:error:/fb:0:0 f:error:/fb:0:0 \
         f2:error:/inner:0:0 g:error:/fb:0:0 h:error:/fb:0:0 n:error:/sn:0:0"
    );
}
