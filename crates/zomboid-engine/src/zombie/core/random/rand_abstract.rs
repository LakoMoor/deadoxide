use crate::java::JavaRandom;

/// zombie.core.random.RandAbstract: the game's range helpers over a `java.util.Random`-shaped generator.
/// `RandStandard`/`RandLua` wrap a CellularAutomatonRNG, `RandSeeded` a plain `java.util.Random`.
pub struct RandAbstract<R: JavaRandom> {
    id: i32,
    pub rand: R,
}

impl<R: JavaRandom> RandAbstract<R> {
    pub fn new(rand: R) -> Self {
        RandAbstract { id: 0, rand }
    }
    fn bump(&mut self) {
        self.id += 1;
        if self.id >= 10000 {
            self.id = 0;
        }
    }
    /// zombie.core.random.RandAbstract.Next (int max)
    pub fn next(&mut self, max: i32) -> i32 {
        if max <= 0 {
            return 0;
        }
        self.bump();
        self.rand.next_int_bound(max)
    }
    /// zombie.core.random.RandAbstract.Next (long max): the original narrows to int first.
    pub fn next_long(&mut self, max: i64) -> i64 {
        self.next(max as i32) as i64
    }
    /// zombie.core.random.RandAbstract.Next (int min, int max): `max` exclusive, arguments in either order.
    pub fn next_range(&mut self, mut min: i32, mut max: i32) -> i32 {
        if max == min {
            return min;
        }
        if min > max {
            std::mem::swap(&mut min, &mut max);
        }
        self.bump();
        self.rand
            .next_int_bound(max.wrapping_sub(min))
            .wrapping_add(min)
    }
    /// zombie.core.random.RandAbstract.Next (float min, float max)
    pub fn next_float(&mut self, mut min: f32, mut max: f32) -> f32 {
        if max == min {
            return min;
        }
        if min > max {
            std::mem::swap(&mut min, &mut max);
        }
        self.bump();
        min + self.rand.next_float() * (max - min)
    }
    /// zombie.core.random.RandInterface.NextBool (int invProbability)
    pub fn next_bool(&mut self, inv_probability: i32) -> bool {
        self.next(inv_probability) == 0
    }
    /// zombie.core.random.Rand.NextInclusive
    pub fn next_inclusive(&mut self, inclusive_min: i32, inclusive_max: i32) -> i32 {
        self.next_range(inclusive_min, inclusive_max.wrapping_add(1))
    }
}
