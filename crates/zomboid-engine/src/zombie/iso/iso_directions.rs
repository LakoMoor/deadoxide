use crate::java;
use crate::zombie::core::math::pz_math;

/// zombie.iso.IsoDirections: ordinal order matters (rotation is index arithmetic).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum IsoDirections {
    N = 0,
    NW = 1,
    W = 2,
    SW = 3,
    S = 4,
    SE = 5,
    E = 6,
    NE = 7,
}
use IsoDirections::*;
pub const VALUES: [IsoDirections; 8] = [N, NW, W, SW, S, SE, E, NE];
const DX: [i32; 8] = [0, -1, -1, -1, 0, 1, 1, 1];
const DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
const NAMES: [&str; 8] = ["N", "NW", "W", "SW", "S", "SE", "E", "NE"];

impl IsoDirections {
    pub fn ordinal(self) -> usize {
        self as usize
    }
    pub fn name(self) -> &'static str {
        NAMES[self as usize]
    }
    /// zombie.iso.IsoDirections.fromString: `None` where `Enum.valueOf` throws. Java's `toUpperCase` is locale-aware;
    /// ASCII is assumed here.
    pub fn from_string(s: &str) -> Option<Self> {
        let up = java::trim(s).to_uppercase();
        NAMES.iter().position(|n| *n == up).map(|i| VALUES[i])
    }
    /// zombie.iso.IsoDirections.fromIndex
    pub fn from_index(index: i32) -> Self {
        VALUES[(index & 7) as usize]
    }
    /// zombie.iso.IsoDirections.RotLeft
    pub fn rot_left(self) -> Self {
        VALUES[(self as usize + 1) & 7]
    }
    /// zombie.iso.IsoDirections.RotLeft (int times)
    pub fn rot_left_times(self, times: i32) -> Self {
        Self::from_index((self as i32).wrapping_add(times))
    }
    /// zombie.iso.IsoDirections.RotRight
    pub fn rot_right(self) -> Self {
        Self::from_index(self as i32 - 1)
    }
    /// zombie.iso.IsoDirections.RotRight (int times)
    pub fn rot_right_times(self, times: i32) -> Self {
        Self::from_index((self as i32).wrapping_sub(times))
    }
    /// zombie.iso.IsoDirections.Rot180
    pub fn rot180(self) -> Self {
        VALUES[(self as usize + 4) & 7]
    }
    /// zombie.iso.IsoDirections.fromAngle (float dx, float dy)
    pub fn from_angle_xy(dx: f32, dy: f32) -> Self {
        Self::safe_from_angle((dy as f64).atan2(dx as f64) as f32)
    }
    /// zombie.iso.IsoDirections.fromAngle (float angleRadians)
    pub fn from_angle(angle_radians: f32) -> Self {
        Self::safe_from_angle(pz_math::wrap(
            angle_radians,
            -std::f32::consts::PI,
            std::f32::consts::PI,
        ))
    }
    /// zombie.iso.IsoDirections.safeFromAngle
    fn safe_from_angle(a: f32) -> Self {
        VALUES[(6i32.wrapping_sub((8.5f32 + 1.2732395f32 * a) as i32) & 7) as usize]
    }
    /// zombie.iso.IsoDirections.cardinalFromAngle (float dx, float dy)
    pub fn cardinal_from_angle_xy(dx: f32, dy: f32) -> Self {
        Self::safe_cardinal_from_angle((dy as f64).atan2(dx as f64) as f32)
    }
    /// zombie.iso.IsoDirections.cardinalFromAngle (float angleRadians)
    pub fn cardinal_from_angle(angle_radians: f32) -> Self {
        Self::safe_cardinal_from_angle(pz_math::wrap(
            angle_radians,
            -std::f32::consts::PI,
            std::f32::consts::PI,
        ))
    }
    /// zombie.iso.IsoDirections.safeCardinalFromAngle
    fn safe_cardinal_from_angle(a: f32) -> Self {
        VALUES[(2 * (3i32.wrapping_sub((4.5f32 + 0.63661975f32 * a) as i32) & 3)) as usize]
    }
    /// zombie.iso.IsoDirections.dx
    pub fn dx(self) -> i32 {
        DX[self as usize]
    }
    /// zombie.iso.IsoDirections.dy
    pub fn dy(self) -> i32 {
        DY[self as usize]
    }
    /// zombie.iso.IsoDirections.isCardinal
    pub fn is_cardinal(self) -> bool {
        self.dx() == 0 || self.dy() == 0
    }
    /// zombie.iso.IsoDirections.isDiagonal
    pub fn is_diagonal(self) -> bool {
        self.dx() != 0 && self.dy() != 0
    }
    /// zombie.iso.IsoDirections.ToVector: (dx, dy) after zombie.iso.Vector2.normalize.
    pub fn to_vector(self) -> (f32, f32) {
        let (x, y) = (self.dx() as f32, self.dy() as f32);
        let length_sq = x * x + y * y;
        if pz_math::equal(length_sq, 1.0, 1.0e-5) {
            (x, y)
        } else {
            let length = (length_sq as f64).sqrt() as f32;
            (x / length, y / length)
        }
    }
    /// zombie.iso.IsoDirections.toAngle
    pub fn to_angle(self) -> f32 {
        self as usize as f32 * (std::f64::consts::PI * 2.0) as f32 / 8.0
    }
    /// zombie.iso.IsoDirections.toAngleDegrees
    pub fn to_angle_degrees(self) -> f32 {
        self.to_angle() * (180.0 / std::f64::consts::PI as f32)
    }
}
