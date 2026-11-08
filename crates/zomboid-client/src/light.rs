//! Point-light falloff as read from the game's native `Lighting64.dll` (see docs/COMPATIBILITY.md,
//! "Per-light contribution in Lighting64.dll"). The formula is an observation of the DLL, NOT yet
//! checked against an oracle frame: no parity is claimed and nothing in the renderer uses it yet.

/// Strength multiplier read from the DLL's constant table (1.15).
const FALLOFF_GAIN: f32 = 1.15;

/// Lighting64.dll FUN_180009600 (non-cone branch): RGB contribution of one light to one square.
/// `d` is the propagated path distance from the light, `radius` the light radius, `intensity` the
/// light's strength and `colour` its RGB in 0..1. Returns integer channels in 0..=~293 (not clamped,
/// as in the DLL), or `None` when the square is outside the radius.
#[allow(dead_code)] // PLACEHOLDER wiring: not used by the renderer until an oracle diff justifies it
pub fn point_light_rgb(d: f32, radius: f32, intensity: f32, colour: [f32; 3]) -> Option<[i32; 3]> {
    if radius <= 0.0 || d >= radius {
        return None;
    }
    let t = (d / radius).clamp(0.0, 1.0);
    let att = (1.0 - t * t) * FALLOFF_GAIN;
    let ch = |c: f32| (att * intensity * c * 255.0) as i32;
    Some([ch(colour[0]), ch(colour[1]), ch(colour[2])])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centre_of_a_white_light_is_over_full_scale() {
        // t = 0: att = 1.15, so 1.15 * 255 = 293.25, truncated to 293.
        assert_eq!(point_light_rgb(0.0, 8.0, 1.0, [1.0; 3]), Some([293; 3]));
    }

    #[test]
    fn halfway_follows_one_minus_t_squared() {
        // t = 0.5: att = 0.75 * 1.15 = 0.8625; 0.8625 * 255 = 219.9 -> 219.
        assert_eq!(point_light_rgb(4.0, 8.0, 1.0, [1.0; 3]), Some([219; 3]));
    }

    #[test]
    fn outside_the_radius_gives_nothing() {
        assert_eq!(point_light_rgb(8.0, 8.0, 1.0, [1.0; 3]), None);
        assert_eq!(point_light_rgb(9.0, 8.0, 1.0, [1.0; 3]), None);
    }

    #[test]
    fn colour_channels_scale_independently() {
        let [r, g, b] = point_light_rgb(0.0, 8.0, 0.5, [1.0, 0.5, 0.0]).unwrap();
        assert_eq!((r, g, b), (146, 73, 0));
    }
}
