//! The crypto randomness syscall (identity.md §8/§11, `bl-3972`/`bl-cf3a`).
//!
//! `crypto.getRandomValues`/`randomUUID` are cheap capabilities frot can provide
//! *genuinely* — real OS randomness, not a deterministic fake — so the JS
//! `crypto.js` prelude fills its buffers from this one syscall. The bytes come
//! from `/dev/urandom` (Linux, matching the persona; the static musl target and
//! the test host are both Linux), so there is no new dependency and no crypto
//! provider to spin up per call. Randomness is deliberately non-deterministic:
//! VISION principle 1's reproducibility is a within-call property (frozen fetches,
//! the virtual clock), and OQ-2 resolved *against* seeding `Math.random`; separate
//! `getRandomValues` calls therefore share no state and cannot be pinned in tests.

use rquickjs::{Ctx, Function, Object};

use super::Host;

macro_rules! bind {
    ($ctx:expr, $g:expr, $name:literal, $f:expr) => {
        $g.set($name, Function::new($ctx.clone(), $f)?.with_name($name)?)?
    };
}

/// Register `__frot_random_bytes(n)` — `n` fresh OS-random bytes as a JS array the
/// `crypto.js` prelude copies into the caller's typed array. The prelude enforces
/// the browser argument/length/error contract (integer-typed view, 65 536-byte
/// quota); this syscall only sources entropy.
pub fn install<'js>(ctx: &Ctx<'js>, g: &Object<'js>, _host: &Host) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_random_bytes", |n: u32| random_bytes(n));
    Ok(())
}

/// `n` bytes of OS randomness from `/dev/urandom`. `read_exact` guarantees a full
/// fill; a failure to read the kernel CSPRNG is unrecoverable and cannot be
/// meaningfully handled or faked, so it panics (the `Engine::new` `.expect`
/// posture) rather than returning short, weak, or predictable bytes.
fn random_bytes(n: u32) -> Vec<u8> {
    use std::io::Read;
    let mut buf = vec![0u8; n as usize];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf))
        .expect("read OS randomness from /dev/urandom");
    buf
}

#[cfg(test)]
mod tests;
