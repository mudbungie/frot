//! A compressed NIST point (`02`/`03` ‖ x) made whole (`bl-0cae`): Firefox
//! accepts one and exports the uncompressed form, and the provider can read
//! one but never writes it back out. y² = x³ − 3x + b over the curve's prime
//! p, and all three primes are 3 mod 4, so y = (x³ − 3x + b)^((p+1)/4) — one
//! modular exponentiation on numbers of at most 521 bits, done here in 32-bit
//! limbs (little-endian). Slow and simple; it runs once per imported key.

type Big = Vec<u32>;

/// A NIST curve's (p, b), big-endian hex; its coordinates are as many bytes
/// as p.
pub type Curve = (&'static str, &'static str);

pub const P256: Curve = (
    "ffffffff00000001000000000000000000000000ffffffffffffffffffffffff",
    "5ac635d8aa3a93e7b3ebbd55769886bc651d06b0cc53b0f63bce3c3e27d2604b",
);
pub const P384: Curve = (
    "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffeffffffff0000000000000000ffffffff",
    "b3312fa7e23ee7e4988e056be3f82d19181d9c6efe8141120314088f5013875ac656398d8a2ed19d2a85c8edd3ec2aef",
);
pub const P521: Curve = (
    "01ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    "0051953eb9618e1c9a1f929a21a0b68540eea2da725b99b315f3b8b489918ef109e156193951ec7e937b1652c0bd3bb1bf073573df883d2c34f1ef451fd46b503f00",
);

fn from_hex(h: &str) -> Big {
    let bytes: Vec<u8> = (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0))
        .collect();
    from_be(&bytes)
}

fn from_be(bytes: &[u8]) -> Big {
    let mut out = vec![0u32; bytes.len().div_ceil(4) + 1];
    for (i, b) in bytes.iter().rev().enumerate() {
        out[i / 4] |= u32::from(*b) << (8 * (i % 4));
    }
    out
}

fn to_be(v: &[u32], n: usize) -> Vec<u8> {
    (0..n)
        .rev()
        .map(|i| (v[i / 4] >> (8 * (i % 4))) as u8)
        .collect()
}

fn bit(v: &[u32], i: usize) -> bool {
    v[i / 32] >> (i % 32) & 1 == 1
}

fn ge(a: &[u32], b: &[u32]) -> bool {
    a.iter().rev().cmp(b.iter().rev()) != std::cmp::Ordering::Less
}

/// a − b in place; a ≥ b, same length.
fn sub(a: &mut [u32], b: &[u32]) {
    let mut borrow = 0i64;
    for (x, y) in a.iter_mut().zip(b) {
        let d = i64::from(*x) - i64::from(*y) - borrow;
        borrow = i64::from(d < 0);
        *x = d.rem_euclid(1 << 32) as u32;
    }
}

/// x mod p, bit by bit from the top: r stays below 2p in p's limbs.
fn reduce(x: &[u32], p: &[u32]) -> Big {
    let mut r = vec![0u32; p.len()];
    for i in (0..x.len() * 32).rev() {
        let mut carry = u32::from(bit(x, i));
        for limb in r.iter_mut() {
            let next = *limb >> 31;
            *limb = *limb << 1 | carry;
            carry = next;
        }
        if ge(&r, p) {
            sub(&mut r, p);
        }
    }
    r
}

fn mulmod(a: &[u32], b: &[u32], p: &[u32]) -> Big {
    let mut out = vec![0u32; a.len() + b.len()];
    for (i, x) in a.iter().enumerate() {
        let mut carry = 0u64;
        for (j, y) in b.iter().enumerate() {
            let t = u64::from(out[i + j]) + u64::from(*x) * u64::from(*y) + carry;
            out[i + j] = t as u32;
            carry = t >> 32;
        }
        out[i + b.len()] = carry as u32;
    }
    reduce(&out, p)
}

/// The uncompressed point (`04` ‖ x ‖ y) for a compressed one (`02`/`03` ‖
/// x) on `curve`; `None` if x is not below p or has no y.
pub fn decompress(curve: Curve, point: &[u8]) -> Option<Vec<u8>> {
    let n = curve.0.len() / 2;
    let (p, b) = (from_hex(curve.0), from_hex(curve.1));
    let x = from_be(&point[1..]);
    // rhs = x³ − 3x + b, as x·x·x + (p − 3)·x + b, all mod p.
    let mut three = p.clone();
    sub(&mut three, &from_be(&[3]));
    let x3 = mulmod(&mulmod(&x, &x, &p), &x, &p);
    let mut rhs = mulmod(&three, &x, &p);
    rhs = reduce(&add(&add(&x3, &rhs), &b), &p);
    // e = (p + 1) / 4.
    let mut e = add(&p, &from_be(&[1]));
    for _ in 0..2 {
        e = halve(&e);
    }
    let mut y = from_be(&[1]);
    for i in (0..e.len() * 32).rev() {
        y = mulmod(&y, &y, &p);
        if bit(&e, i) {
            y = mulmod(&y, &rhs, &p);
        }
    }
    if mulmod(&y, &y, &p) != rhs || ge(&x, &p) {
        return None;
    }
    if u8::from(bit(&y, 0)) != point[0] & 1 && y.iter().any(|l| *l != 0) {
        let mut neg = p.clone();
        sub(&mut neg, &y);
        y = neg;
    }
    let mut out = vec![4];
    out.extend(to_be(&x, n));
    out.extend(to_be(&y, n));
    Some(out)
}

fn add(a: &[u32], b: &[u32]) -> Big {
    let mut out = vec![0u32; a.len().max(b.len()) + 1];
    let mut carry = 0u64;
    for (i, o) in out.iter_mut().enumerate() {
        let t = u64::from(*a.get(i).unwrap_or(&0)) + u64::from(*b.get(i).unwrap_or(&0)) + carry;
        *o = t as u32;
        carry = t >> 32;
    }
    out
}

fn halve(a: &[u32]) -> Big {
    (0..a.len())
        .map(|i| a[i] >> 1 | a.get(i + 1).map_or(0, |n| n << 31))
        .collect()
}
