//! The `pattern` attribute's regular expression (`bl-51e9`): HTML compiles it
//! as `^(?:pattern)$` with the `v` flag — a JavaScript regular expression —
//! so it is matched by the engine's own `RegExp`, never a second regex
//! implementation. Each call builds a fresh runtime holding nothing but the
//! base objects and `RegExp`: no page, no prelude, no `--js` — the answer is a
//! pure function of the two strings, and nothing outlives the call.

use std::time::{Duration, Instant};

use rquickjs::context::intrinsic::{RegExp, RegExpCompiler};
use rquickjs::function::This;
use rquickjs::{Constructor, Context, Function, Object, Runtime};

/// How long one match may run before it is abandoned (a pattern that
/// backtracks without end), and the pattern treated as not compiling.
const LIMIT: Duration = Duration::from_millis(100);

/// Whether `value` matches `pattern` whole, or `None` when the pattern does not
/// compile (or runs past [`LIMIT`]) — HTML then ignores it.
pub fn pattern_matches(pattern: &str, value: &str) -> Option<bool> {
    let rt = Runtime::new().expect("quickjs runtime");
    let started = Instant::now();
    rt.set_interrupt_handler(Some(Box::new(move || started.elapsed() > LIMIT)));
    let ctx = Context::custom::<(RegExpCompiler, RegExp)>(&rt).expect("quickjs context");
    ctx.with(|ctx| {
        let regexp: Constructor = ctx.globals().get("RegExp").expect("RegExp intrinsic");
        let source = format!("^(?:{pattern})$");
        let re: Object = regexp.construct((source, "v")).ok()?;
        let test: Function = re.get("test").expect("RegExp.prototype.test");
        test.call((This(re), value)).ok()
    })
}

#[cfg(test)]
mod tests {
    use super::pattern_matches;

    #[test]
    fn matches_whole_values_and_ignores_what_does_not_compile() {
        assert_eq!(pattern_matches("[0-9]+", "12"), Some(true));
        assert_eq!(pattern_matches("[0-9]+", "1a"), Some(false));
        // Anchored as a whole: an alternation cannot match a prefix.
        assert_eq!(pattern_matches("a|ab", "ab"), Some(true));
        assert_eq!(pattern_matches("[", "x"), None);
        // A runaway backtrack is abandoned, not waited out.
        let long = format!("{}!", "a".repeat(40));
        assert_eq!(pattern_matches("(a+)+", &long), None);
    }
}
