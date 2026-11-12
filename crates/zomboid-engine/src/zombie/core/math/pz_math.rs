use crate::java;
use crate::zombie::util::string_utils;

/// zombie.core.math.PZMath.clamp (int).
pub fn clamp_i32(val: i32, min: i32, max: i32) -> i32 {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

/// zombie.core.math.PZMath.clamp (float). NaN passes through, as in Java.
pub fn clamp(val: f32, min: f32, max: f32) -> f32 {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

/// zombie.core.math.PZMath.clamp_01.
pub fn clamp_01(val: f32) -> f32 {
    clamp(val, 0.0, 1.0)
}

/// zombie.core.math.PZMath.tryParseInt.
pub fn try_parse_int(var_str: Option<&str>, default_val: i32) -> i32 {
    if string_utils::is_null_or_whitespace(var_str) {
        return default_val;
    }
    java::parse_int(java::trim(var_str.unwrap_or(""))).unwrap_or(default_val)
}

/// zombie.core.math.PZMath.fastfloor (float).
pub fn fastfloor(val: f32) -> i32 {
    let xi = val as i32;
    if val < xi as f32 {
        xi - 1
    } else {
        xi
    }
}

/// zombie.core.math.PZMath.fastfloor (double).
pub fn fastfloor_f64(val: f64) -> i32 {
    let xi = val as i32;
    if val < xi as f64 {
        xi - 1
    } else {
        xi
    }
}

/// zombie.core.math.PZMath.coorddivision.
pub fn coorddivision(value: i32, divisor: i32) -> i32 {
    fastfloor(value as f32 / divisor as f32)
}

/// zombie.core.math.PZMath.coordmodulo.
pub fn coordmodulo(value: i32, divisor: i32) -> i32 {
    value - fastfloor(value as f32 / divisor as f32) * divisor
}

/// zombie.core.math.PZMath.equal (float, float, float).
pub fn equal(a: f32, b: f32, delta: f32) -> bool {
    (b - a).abs() < delta
}

/// zombie.core.math.PZMath.max (float).
pub fn max(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

/// zombie.core.math.PZMath.min (float).
pub fn min(a: f32, b: f32) -> f32 {
    if a > b {
        b
    } else {
        a
    }
}

/// zombie.core.math.PZMath.sign (float).
pub fn sign(val: f32) -> i32 {
    if val > 0.0 {
        1
    } else if val < 0.0 {
        -1
    } else {
        0
    }
}

/// zombie.core.math.PZMath.floor (float).
pub fn floor(val: f32) -> f32 {
    fastfloor(val) as f32
}

/// zombie.core.math.PZMath.frac.
pub fn frac(val: f32) -> f32 {
    val - floor(val)
}

/// zombie.core.math.PZMath.wrap (float val, float range).
pub fn wrap_range(val: f32, range: f32) -> f32 {
    if range <= 0.0 {
        0.0
    } else if val < 0.0 {
        let multiple_negative = -val / range;
        let frac_up = 1.0 - frac(multiple_negative);
        frac_up * range
    } else {
        frac(val / range) * range
    }
}

/// zombie.core.math.PZMath.wrap (float val, float min, float max).
pub fn wrap(val: f32, min_v: f32, max_v: f32) -> f32 {
    let real_max = max(max_v, min_v);
    let real_min = min(max_v, min_v);
    let range = real_max - real_min;
    let rel_val = val - real_min;
    real_min + wrap_range(rel_val, range)
}
