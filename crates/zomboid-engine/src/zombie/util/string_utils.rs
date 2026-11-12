use crate::java;

/// zombie.util.StringUtils.isNullOrWhitespace: `None` is Java null. The original's two-ended scan is kept as written.
pub fn is_null_or_whitespace(s: Option<&str>) -> bool {
    match s {
        None => true,
        Some("") => true,
        Some(s) => is_whitespace(s),
    }
}

/// zombie.util.StringUtils.isWhitespace.
fn is_whitespace(s: &str) -> bool {
    let c: Vec<char> = s.chars().collect();
    let length = c.len();
    if length == 0 {
        return false;
    }
    let (mut start, middle, mut end) = (0usize, length / 2, length - 1);
    while start <= middle {
        if !java::is_whitespace(c[start]) || !java::is_whitespace(c[end]) {
            return false;
        }
        start += 1;
        end = end.wrapping_sub(1);
    }
    true
}

/// zombie.util.StringUtils.equalsIgnoreCase: null and empty compare equal to each other.
pub fn equals_ignore_case(a: Option<&str>, b: Option<&str>) -> bool {
    let empty = |s: Option<&str>| s.is_none_or(str::is_empty);
    if empty(a) && empty(b) {
        return true;
    }
    match (a, b) {
        (Some(a), Some(b)) => java::equals_ignore_case(a, b),
        _ => false,
    }
}
