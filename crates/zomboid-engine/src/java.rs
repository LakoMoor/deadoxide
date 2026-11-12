//! JDK behaviours the game's code relies on, reproduced where Rust's std differs.

/// java.lang.String.trim: strips code units `<= U+0020` from both ends (not Unicode whitespace).
pub fn trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

/// java.lang.Integer.parseInt(String): optional sign then decimal digits, no whitespace; `None` is NumberFormatException.
/// Non-ASCII digits, which Java also accepts, are rejected here.
pub fn parse_int(s: &str) -> Option<i32> {
    if s.is_empty()
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'+' || b == b'-')
    {
        return None;
    }
    s.parse::<i32>().ok()
}

/// java.lang.Character.isWhitespace(char).
pub fn is_whitespace(c: char) -> bool {
    matches!(c, '\t'..='\r' | '\u{1c}'..='\u{1f}')
        || (matches!(
            c,
            ' ' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{205f}' | '\u{3000}'
        ) && c != '\u{2007}')
}

/// java.lang.String.equalsIgnoreCase: per-char comparison after toUpperCase, then toLowerCase.
pub fn equals_ignore_case(a: &str, b: &str) -> bool {
    let (mut x, mut y) = (a.chars(), b.chars());
    loop {
        match (x.next(), y.next()) {
            (None, None) => return true,
            (Some(p), Some(q)) => {
                if p == q {
                    continue;
                }
                let (pu, qu) = (upper(p), upper(q));
                if pu != qu && lower(pu) != lower(qu) {
                    return false;
                }
            }
            _ => return false,
        }
    }
}
fn upper(c: char) -> char {
    let mut u = c.to_uppercase();
    match (u.next(), u.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}
fn lower(c: char) -> char {
    let mut u = c.to_lowercase();
    match (u.next(), u.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

/// java.util.Random: the 48-bit linear congruential generator, bit-for-bit.
#[derive(Clone, Debug)]
pub struct Random {
    seed: i64,
}
const MULTIPLIER: i64 = 0x5DEECE66D;
const MASK: i64 = (1 << 48) - 1;
impl Random {
    /// java.util.Random(long seed).
    pub fn new(seed: i64) -> Self {
        Random {
            seed: (seed ^ MULTIPLIER) & MASK,
        }
    }
    /// java.util.Random.next(int bits).
    pub fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(0xB)) & MASK;
        (self.seed >> (48 - bits)) as i32
    }
    /// java.util.Random.nextInt().
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }
    /// java.util.Random.nextInt(int bound), the legacy algorithm kept by JDK 17+ for java.util.Random itself. `bound` must be positive.
    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let mut r = self.next(31);
        let m = bound.wrapping_sub(1);
        if bound & m == 0 {
            ((bound as i64).wrapping_mul(r as i64) >> 31) as i32
        } else {
            let mut u = r;
            loop {
                r = u % bound;
                if u.wrapping_sub(r).wrapping_add(m) >= 0 {
                    break;
                }
                u = self.next(31);
            }
            r
        }
    }
    /// java.util.Random.nextLong().
    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }
    /// java.util.Random.nextFloat().
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1 << 24) as f32
    }
    /// java.util.Random.nextDouble().
    pub fn next_double(&mut self) -> f64 {
        (((self.next(26) as i64) << 27) + self.next(27) as i64) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    /// java.util.Random.nextBoolean().
    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }
}

/// The source of randomness behind `RandAbstract`: the two calls the game makes on a `java.util.Random`.
pub trait JavaRandom {
    fn next_int_bound(&mut self, bound: i32) -> i32;
    fn next_float(&mut self) -> f32;
}
impl JavaRandom for Random {
    fn next_int_bound(&mut self, bound: i32) -> i32 {
        Random::next_int_bound(self, bound)
    }
    fn next_float(&mut self) -> f32 {
        Random::next_float(self)
    }
}
