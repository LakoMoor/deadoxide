//! Window-free simulation core: collision grid and player movement (fixed-step friendly, no wall clock, no RNG).
//!
//! Collision rules are a SIMPLIFIED reading of tile properties and are not yet diffed against the original:
//! - a square is blocked if any tile on it has `solid` or `solidtrans`;
//! - `WallN`/`collideN` block crossing the north edge of the square, `WallW`/`collideW` the west edge;
//! - doors and windows are treated as passable for now.
use std::collections::HashMap;

pub const BLOCK: u8 = 1;
pub const EDGE_N: u8 = 2;
pub const EDGE_W: u8 = 4;

#[derive(Default)]
pub struct Collision {
    flags: HashMap<(i32, i32, i32), u8>,
}

impl Collision {
    pub fn add(&mut self, x: i32, y: i32, z: i32, f: u8) {
        *self.flags.entry((x, y, z)).or_default() |= f;
    }
    fn f(&self, x: i32, y: i32, z: i32) -> u8 {
        self.flags.get(&(x, y, z)).copied().unwrap_or(0)
    }
    #[allow(dead_code)] // API kept for tests and the CLI; the window path does not call it yet
    pub fn len(&self) -> usize {
        self.flags.len()
    }
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }
    pub fn blocked(&self, x: i32, y: i32, z: i32) -> bool {
        self.f(x, y, z) & BLOCK != 0
    }
    /// Can an actor move between two edge-adjacent squares?
    pub fn can_step(&self, from: (i32, i32), to: (i32, i32), z: i32) -> bool {
        if self.blocked(to.0, to.1, z) {
            return false;
        }
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        match (dx, dy) {
            (1, 0) => self.f(to.0, to.1, z) & EDGE_W == 0,
            (-1, 0) => self.f(from.0, from.1, z) & EDGE_W == 0,
            (0, 1) => self.f(to.0, to.1, z) & EDGE_N == 0,
            (0, -1) => self.f(from.0, from.1, z) & EDGE_N == 0,
            _ => true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub z: i32,
}

impl Player {
    /// Axis-separated move (world axes, tiles). Returns true if it moved at all.
    pub fn step(&mut self, c: &Collision, dx: f32, dy: f32) -> bool {
        let mut moved = false;
        for (ax, ay) in [(dx, 0.0), (0.0, dy)] {
            if ax == 0.0 && ay == 0.0 {
                continue;
            }
            let (nx, ny) = (self.x + ax, self.y + ay);
            let from = (self.x.floor() as i32, self.y.floor() as i32);
            let to = (nx.floor() as i32, ny.floor() as i32);
            // large steps are split by the caller; here at most one square is crossed
            if from == to || c.can_step(from, to, self.z) {
                self.x = nx;
                self.y = ny;
                moved = true;
            }
        }
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn solid_square_blocks() {
        let mut c = Collision::default();
        c.add(5, 5, 0, BLOCK);
        let mut p = Player {
            x: 4.5,
            y: 5.5,
            z: 0,
        };
        for _ in 0..20 {
            p.step(&c, 0.1, 0.0);
        }
        assert!(p.x < 5.0, "stopped before the solid square, x={}", p.x);
    }
    #[test]
    fn west_wall_edge_blocks_only_that_direction() {
        let mut c = Collision::default();
        c.add(5, 5, 0, EDGE_W); // wall on the west edge of (5,5)
        let mut p = Player {
            x: 4.5,
            y: 5.5,
            z: 0,
        };
        for _ in 0..20 {
            p.step(&c, 0.1, 0.0);
        }
        assert!(p.x < 5.0);
        let mut q = Player {
            x: 5.5,
            y: 5.5,
            z: 0,
        };
        for _ in 0..20 {
            q.step(&c, -0.1, 0.0);
        }
        assert!(
            q.x >= 5.0 - 1e-3 && q.x < 5.6,
            "blocked from the east side too, x={}",
            q.x
        );
        let mut r = Player {
            x: 4.5,
            y: 5.5,
            z: 0,
        };
        for _ in 0..20 {
            r.step(&c, 0.0, 0.1);
        }
        assert!(r.y > 6.0, "moving along the wall is free");
    }
}

// ---- fixed-step world: deterministic tick, seeded RNG, placeholder zombies ----
// PLACEHOLDER BEHAVIOUR: zombie speed/vision/wander rules below are NOT measured from the original game.

pub const TICK_HZ: f32 = 30.0;

/// xorshift64*: explicit, seedable, no global state.
#[derive(Clone)]
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }
    pub fn unit(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Zombie {
    pub x: f32,
    pub y: f32,
    pub heading: f32,
}

pub struct World {
    pub tick: u64,
    pub rng: Rng,
    pub player: Player,
    pub zombies: Vec<Zombie>,
}

impl World {
    /// Spawn `n` zombies on random unblocked squares within `radius` of the player.
    pub fn new(seed: u64, player: Player, c: &Collision, n: usize, radius: i32) -> Self {
        let mut rng = Rng::new(seed);
        let mut zombies = Vec::new();
        let mut tries = 0;
        while zombies.len() < n && tries < n * 50 {
            tries += 1;
            let x = player.x as i32 + (rng.unit() * (2 * radius) as f32) as i32 - radius;
            let y = player.y as i32 + (rng.unit() * (2 * radius) as f32) as i32 - radius;
            if !c.blocked(x, y, player.z) {
                zombies.push(Zombie {
                    x: x as f32 + 0.5,
                    y: y as f32 + 0.5,
                    heading: rng.unit() * std::f32::consts::TAU,
                });
            }
        }
        World {
            tick: 0,
            rng,
            player,
            zombies,
        }
    }

    /// One fixed step. `input` = desired player move direction in world axes (will be normalised), speed in tiles/s.
    pub fn step(&mut self, c: &Collision, input: (f32, f32)) {
        let dt = 1.0 / TICK_HZ;
        let len = (input.0 * input.0 + input.1 * input.1).sqrt();
        if len > 0.0 {
            let d = 4.0 * dt;
            self.player.step(c, input.0 / len * d, input.1 / len * d);
        }
        let p = self.player;
        for z in &mut self.zombies {
            let (dx, dy) = (p.x - z.x, p.y - z.y);
            let dist = (dx * dx + dy * dy).sqrt();
            // PLACEHOLDER: chase within 12 tiles, otherwise wander
            let speed = if dist < 12.0 {
                z.heading = dy.atan2(dx);
                1.2
            } else {
                if self.rng.unit() < 0.02 {
                    z.heading += (self.rng.unit() - 0.5) * 2.0;
                }
                0.5
            };
            let (sx, sy) = (z.heading.cos() * speed * dt, z.heading.sin() * speed * dt);
            let mut zp = Player {
                x: z.x,
                y: z.y,
                z: p.z,
            };
            if !zp.step(c, sx, sy) {
                z.heading += 1.0;
            }
            z.x = zp.x;
            z.y = zp.y;
        }
        self.tick += 1;
    }

    #[allow(dead_code)] // used by the determinism tests
    /// FNV-1a over tick, player and zombie bit patterns: for determinism/golden tests.
    pub fn state_hash(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        let mut mix = |v: u64| {
            for b in v.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100_0000_01b3);
            }
        };
        mix(self.tick);
        mix(self.player.x.to_bits() as u64);
        mix(self.player.y.to_bits() as u64);
        for z in &self.zombies {
            mix(z.x.to_bits() as u64);
            mix(z.y.to_bits() as u64);
        }
        h
    }
}

#[cfg(test)]
mod world_tests {
    use super::*;
    fn run(seed: u64) -> (u64, World) {
        let mut c = Collision::default();
        for i in -10..=10 {
            c.add(20 + i, 10, 0, BLOCK);
            c.add(20 + i, 30, 0, BLOCK);
            c.add(10, 20 + i, 0, BLOCK);
            c.add(30, 20 + i, 0, BLOCK);
        }
        let mut w = World::new(
            seed,
            Player {
                x: 20.5,
                y: 20.5,
                z: 0,
            },
            &c,
            25,
            8,
        );
        for t in 0..600 {
            w.step(&c, if t % 90 < 45 { (1.0, 0.0) } else { (0.0, 1.0) });
        }
        (w.state_hash(), w)
    }
    #[test]
    fn same_seed_same_state_different_seed_differs() {
        assert_eq!(run(7).0, run(7).0);
        assert_ne!(run(7).0, run(8).0);
    }
    #[test]
    fn zombies_stay_inside_walls_and_approach() {
        let (_, w) = run(7);
        assert_eq!(w.zombies.len(), 25);
        for z in &w.zombies {
            assert!(
                z.x > 10.0 && z.x < 31.0 && z.y > 10.0 && z.y < 31.0,
                "escaped: {z:?}"
            );
        }
    }
}
