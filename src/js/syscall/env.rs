//! Environment syscalls (js.md §7) — the static facts the JS `env.js` prelude
//! needs to build `navigator`/`location`/`matchMedia`, plus the counted-no-op
//! sink for navigation the shim refuses (assigning `location`, §11).
//!
//! URL decomposition is done here, by the `url` crate, so the parser stays the
//! single source of truth for URL semantics; the prelude never re-parses.

use rquickjs::{Ctx, Function, Object};
use url::Url;

use super::Host;
use crate::fetch::FIREFOX_140_ESR;
use crate::layout::{VIEWPORT_HEIGHT, VIEWPORT_WIDTH};

/// Bind one named host function, keeping the `?` error paths on the call line so
/// line coverage sees the executed (never-erroring) `set` — the same shape the
/// core table's `bind!` macro uses.
macro_rules! bind {
    ($ctx:expr, $g:expr, $name:literal, $f:expr) => {
        $g.set($name, Function::new($ctx.clone(), $f)?.with_name($name)?)?
    };
}

/// Register `__frot_env_profile`, `__frot_location`, `__frot_url_parse`,
/// `__frot_viewport_width`, `__frot_viewport_height`, `__frot_media_matches`,
/// and `__frot_denied` on the realm. Clones the static facts out of `host.env`
/// into the closures; `host.counters.denials` is the shared counter the prelude
/// bumps on a refused navigation. `__frot_media_matches` backs `matchMedia` with
/// the CSS engine's media-query evaluator (`css::media`) — the same authority
/// `@media` blocks cascade through, so CSS and JS can never disagree.
///
/// `__frot_env_profile` is the *one* channel the JS persona (`bl-3972`,
/// identity.md §4/§8) draws every navigator/screen/Intl fact from — the way the
/// UA already flowed — so no identity literal is duplicated in the prelude (I1).
/// The effective UA and Accept-Language ride from `host.env` (a `-H` override
/// updates only those, I5); everything else derives from the [`FIREFOX_140_ESR`]
/// SSOT, so HTTP and JS cannot contradict each other.
pub fn install<'js>(ctx: &Ctx<'js>, g: &Object<'js>, host: &Host) -> rquickjs::Result<()> {
    let denials = &host.counters.denials;
    let (ua, lang) = (
        host.env.user_agent.clone(),
        host.env.accept_language.clone(),
    );
    bind!(ctx, g, "__frot_env_profile", move || persona(&ua, &lang));
    // The observable browser clock's raw source (`bl-e707`): real elapsed ms on
    // the session's one monotonic clock — the same clock the §5/§6 deadline bounds
    // the run with. `loop.js` adds the virtual timer offset and floors to the
    // profile precision on top of this, so `performance.now`/`Date.now` advance
    // with actual CPU/host/network time and never see perpetual zero.
    let clock = host.clock.clone();
    bind!(ctx, g, "__frot_now", move || {
        clock.elapsed_nanos() as f64 / 1_000_000.0
    });
    let url = host.env.url.clone();
    bind!(
        ctx,
        g,
        "__frot_location",
        move |ctx: Ctx<'js>| location_obj(&ctx, &url)
    );
    bind!(
        ctx,
        g,
        "__frot_url_parse",
        |ctx: Ctx<'js>, spec: String, base: Option<String>| {
            url_parse(&ctx, &spec, base.as_deref())
        }
    );
    bind!(ctx, g, "__frot_viewport_width", || VIEWPORT_WIDTH);
    bind!(ctx, g, "__frot_viewport_height", || VIEWPORT_HEIGHT);
    bind!(ctx, g, "__frot_media_matches", |q: String| {
        crate::css::media::matches(&q)
    });
    let d = denials.clone();
    bind!(ctx, g, "__frot_denied", move || *d.borrow_mut() += 1);
    Ok(())
}

/// The persona facts the JS prelude reads, as a JSON string (`bl-3972`,
/// identity.md §4/§8) — the single derivation site for every JS-visible identity
/// fact, so `navigator`/`screen`/`Intl` carry no literals of their own (I1). The
/// prelude does one `JSON.parse`. Returned as a serialized payload rather than a
/// field-by-field rquickjs `Object` deliberately: the value types are mixed
/// (string/number/bool/array) and several (`""`, `0`, `true`) do not heap-
/// allocate, so a per-field `Object::set` would carry `?` error edges that no
/// starvation test could ever reach (dead regions). One `serde_json` build has a
/// single serialize edge (infallible for this fixed shape — the [`envelope`]
/// pattern), and stays a pure, directly-testable function.
///
/// The pinned facts come from [`FIREFOX_140_ESR`]; `userAgent`/`appVersion` and
/// the `language(s)` come from the *effective* `ua`/`accept_language` (a `-H`
/// override touches only those, I5), guaranteeing HTTP↔JS coherence. Screen
/// geometry is *not* here — it stays the layout viewport constant (§8).
///
/// [`envelope`]: crate::envelope
fn persona(ua: &str, accept_language: &str) -> String {
    let p = &FIREFOX_140_ESR;
    let langs = languages(accept_language);
    let facts = serde_json::json!({
        "userAgent": ua,
        "appVersion": ua.strip_prefix("Mozilla/").unwrap_or(ua),
        "appName": "Netscape",
        "appCodeName": "Mozilla",
        "product": "Gecko",
        "productSub": p.product_sub,
        "vendor": "",
        "vendorSub": "",
        "platform": p.platform,
        "oscpu": p.platform,
        "language": langs.first().map(String::as_str).unwrap_or(""),
        "languages": langs,
        "doNotTrack": "unspecified",
        "buildID": p.build_id,
        "hardwareConcurrency": p.hardware_concurrency,
        "maxTouchPoints": 0,
        "cookieEnabled": true,
        "onLine": true,
        "pdfViewerEnabled": true,
        "colorDepth": p.color_depth,
        // The FIXED 2D-canvas digest seed (bl-05e6, identity.md §11): the one
        // channel canvas.js draws its determinism from. A profile const, never
        // host entropy, so `toDataURL`/`getImageData` match across invocations.
        "canvasSeed": p.canvas_seed,
        // A borderless 1280×720 desktop viewport (layout.rs) implies DPR 1 — the
        // CSS px and device px are one. Not a persona field: a fact of that model.
        "devicePixelRatio": 1,
        "locale": p.language,
        // Timezone pinned to UTC on the same determinism argument as
        // `hardwareConcurrency` (identity.md §9): a host-derived zone would leak
        // entropy and vary output. A declared residual (§11), coherent with locale.
        "timeZone": "UTC",
        // Timer precision in microseconds (`bl-e707`, identity.md §9): the one
        // literal `loop.js` floors the observable clock to (1 ms). Derived from
        // the profile SSOT, never a second hardcode in the prelude.
        "timerPrecisionUs": p.timer_precision_us,
    });
    serde_json::to_string(&facts).expect("persona facts are always serializable")
}

/// Split an `Accept-Language` value into its ordered language tags, dropping the
/// `;q=` weights — so `en-US,en;q=0.5` yields `["en-US","en"]`, exactly the
/// `navigator.languages` a real Firefox reports for that header. Deriving both
/// from the one string is what makes them incapable of disagreeing (§8).
fn languages(accept_language: &str) -> Vec<String> {
    accept_language
        .split(',')
        .map(|part| part.split(';').next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Decompose a parsed [`Url`] into the standard read-only members. The single
/// source of URL semantics for both `location` and `new URL` (js.md §7) — the
/// prelude never re-parses.
fn decompose<'js>(obj: &Object<'js>, u: &Url) -> rquickjs::Result<()> {
    let host = u.host_str().unwrap_or("");
    let hostport = match u.port() {
        Some(p) => format!("{host}:{p}"),
        None => host.to_string(),
    };
    obj.set("href", u.as_str())?;
    obj.set("protocol", format!("{}:", u.scheme()))?;
    obj.set("host", hostport)?;
    obj.set("hostname", host)?;
    obj.set("port", u.port().map(|p| p.to_string()).unwrap_or_default())?;
    obj.set("pathname", u.path())?;
    obj.set(
        "search",
        u.query().map(|q| format!("?{q}")).unwrap_or_default(),
    )?;
    obj.set(
        "hash",
        u.fragment().map(|f| format!("#{f}")).unwrap_or_default(),
    )?;
    obj.set("origin", u.origin().ascii_serialization())?;
    Ok(())
}

/// Build the `location` object. A parseable URL is decomposed into the standard
/// read-only members; an unparseable one still yields `href` (empty components),
/// never a throw — `location` is a fact, not a denial.
fn location_obj<'js>(ctx: &Ctx<'js>, raw: &str) -> rquickjs::Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;
    match Url::parse(raw) {
        Ok(u) => decompose(&obj, &u)?,
        Err(_) => {
            obj.set("href", raw)?;
            for k in [
                "protocol", "host", "hostname", "port", "pathname", "search", "hash", "origin",
            ] {
                obj.set(k, "")?;
            }
        }
    }
    Ok(obj)
}

/// Back the prelude's `new URL(spec, base)`. Unlike `location` (a fact that
/// empties on failure), an invalid URL is a `TypeError`: the object carries a
/// `valid` flag the prelude converts to a throw. Base resolution rides the `url`
/// crate's `Url::join`, so relative-URL semantics stay in one place (js.md §7).
fn url_parse<'js>(ctx: &Ctx<'js>, spec: &str, base: Option<&str>) -> rquickjs::Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;
    let parsed = match base {
        Some(b) => Url::parse(b).ok().and_then(|u| u.join(spec).ok()),
        None => Url::parse(spec).ok(),
    };
    match parsed {
        Some(u) => {
            obj.set("valid", true)?;
            decompose(&obj, &u)?;
        }
        None => obj.set("valid", false)?,
    }
    Ok(obj)
}

#[cfg(test)]
mod starve_tests;
#[cfg(test)]
mod tests;
