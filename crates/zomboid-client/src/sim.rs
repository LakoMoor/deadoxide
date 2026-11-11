//! Window-free simulation core: collision grid and player movement (fixed-step friendly, no wall clock, no RNG).
//!
//! Collision rules are SIMPLIFIED; static flags have limited original-side checks:
//! - `solid` blocks entry; `solidtrans` does unless the square has a window flag;
//! - a square with both `water` and `solidtrans` also blocks movement out;
//! - `collideN` blocks the north edge and `collideW` the west edge;
//! - diagonal static square checks cover four edges; continuous movement remains approximate;
//! - door and window object state and stairs are not implemented.
use std::collections::HashMap;

pub const BLOCK: u16 = 1;
pub const EDGE_N: u16 = 2;
pub const EDGE_W: u16 = 4;
pub const WATER: u16 = 8;
pub const SOLID_TRANS: u16 = 16;
pub const SOLID: u16 = 32;
pub const WINDOW: u16 = 64;
pub const WINDOW_N: u16 = 128;
pub const WINDOW_W: u16 = 256;
pub const HOPPABLE: u16 = 512;

#[derive(Default)]
pub struct Collision {
    flags: HashMap<(i32, i32, i32), u16>,
}

impl Collision {
    pub fn add(&mut self, x: i32, y: i32, z: i32, f: u16) {
        *self.flags.entry((x, y, z)).or_default() |= f;
    }
    fn f(&self, x: i32, y: i32, z: i32) -> u16 {
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
    /// Experimental line of sight using the movement collision grid; door/window transparency is not modelled.
    /// EXPERIMENT: not yet diffed against the real game beyond three frames.
    pub fn los(&self, from: (f32, f32), to: (i32, i32), z: i32) -> bool {
        let (tx, ty) = (to.0 as f32 + 0.5, to.1 as f32 + 0.5);
        let (dx, dy) = (tx - from.0, ty - from.1);
        let n = ((dx.abs().max(dy.abs())) * 8.0).ceil().max(1.0) as i32;
        let mut cur = (from.0.floor() as i32, from.1.floor() as i32);
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let sq = (
                (from.0 + dx * t).floor() as i32,
                (from.1 + dy * t).floor() as i32,
            );
            if sq == cur {
                continue;
            }
            let ok = if sq.0 != cur.0 && sq.1 != cur.1 {
                let a = (sq.0, cur.1);
                let b = (cur.0, sq.1);
                (self.can_step(cur, a, z) && self.can_step(a, sq, z))
                    || (self.can_step(cur, b, z) && self.can_step(b, sq, z))
            } else {
                self.can_step(cur, sq, z)
            };
            // the target square itself is seen even if it is a wall square
            if !ok && sq != to {
                return false;
            }
            if !ok {
                return true;
            }
            cur = sq;
        }
        true
    }
    fn edge_open(&self, from: (i32, i32), to: (i32, i32), z: i32) -> bool {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        match (dx, dy) {
            (1, 0) => self.f(to.0, to.1, z) & EDGE_W == 0,
            (-1, 0) => self.f(from.0, from.1, z) & EDGE_W == 0,
            (0, 1) => self.f(to.0, to.1, z) & EDGE_N == 0,
            (0, -1) => self.f(from.0, from.1, z) & EDGE_N == 0,
            _ => true,
        }
    }

    /// zombie.iso.IsoGridSquare.CalculateCollide: partial static adjacent-square collision check.
    pub fn can_step(&self, from: (i32, i32), to: (i32, i32), z: i32) -> bool {
        let from_flags = self.f(from.0, from.1, z);
        let destination = self.f(to.0, to.1, z);
        let south = self.f(to.0, to.1 + 1, z);
        let east = self.f(to.0 + 1, to.1, z);
        let near_window = destination & (WINDOW | HOPPABLE) != 0
            || south & (WINDOW_N | HOPPABLE) != 0
            || east & (WINDOW_W | HOPPABLE) != 0;
        let blocked_destination = destination & SOLID != 0
            || (destination & SOLID_TRANS != 0 && !near_window)
            // The BLOCK bit is also used by synthetic and placeholder callers.
            || (destination & BLOCK != 0 && destination & (SOLID | SOLID_TRANS) == 0);
        if blocked_destination
            || from_flags & (WATER | SOLID_TRANS) == (WATER | SOLID_TRANS)
            || destination & (WATER | SOLID_TRANS) == (WATER | SOLID_TRANS)
        {
            return false;
        }
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        if dx.abs() == 1 && dy.abs() == 1 {
            // The original's static diagonal check tests all four edges, but
            // does not treat a solid side square as a solid destination.
            let x_side = (to.0, from.1);
            let y_side = (from.0, to.1);
            self.edge_open(from, x_side, z)
                && self.edge_open(from, y_side, z)
                && self.edge_open(x_side, to, z)
                && self.edge_open(y_side, to, z)
        } else {
            self.edge_open(from, to, z)
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
        let origin = (self.x.floor() as i32, self.y.floor() as i32);
        let diagonal_target = ((self.x + dx).floor() as i32, (self.y + dy).floor() as i32);
        let crosses_diagonal =
            (origin.0 - diagonal_target.0).abs() == 1 && (origin.1 - diagonal_target.1).abs() == 1;
        for (ax, ay) in [(dx, 0.0), (0.0, dy)] {
            if ax == 0.0 && ay == 0.0 {
                continue;
            }
            let (nx, ny) = (self.x + ax, self.y + ay);
            let from = (self.x.floor() as i32, self.y.floor() as i32);
            let to = (nx.floor() as i32, ny.floor() as i32);
            // large steps are split by the caller; here at most one square is crossed
            // PLACEHOLDER: retain axis-separated sliding; exact character
            // collision response has not been traced in the original.
            let diagonal_ok = !(crosses_diagonal
                && to == diagonal_target
                && from != origin
                && !c.can_step(origin, diagonal_target, self.z));
            if diagonal_ok && (from == to || c.can_step(from, to, self.z)) {
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
    fn diagonal_static_flags_match_original_oracle() {
        // Four directions x 4096 flag combinations, generated by DiagonalProbe.
        let oracle = include_str!("../../../tests/golden/collision_diagonal.txt");
        let mut checked = 0;
        for line in oracle.lines() {
            let (direction, bits) = line.split_once(':').expect("oracle line");
            let (dx, dy) = direction.split_once(',').expect("oracle direction");
            let dx: i32 = dx.parse().expect("dx");
            let dy: i32 = dy.parse().expect("dy");
            assert_eq!(bits.len(), 4096);
            for (mask, expected) in bits.bytes().enumerate() {
                let mut c = Collision::default();
                for (i, (x, y)) in [(0, 0), (dx, 0), (0, dy), (dx, dy)].into_iter().enumerate() {
                    c.add(x, y, 0, ((mask >> (3 * i)) & 7) as u16);
                }
                assert_eq!(
                    !c.can_step((0, 0), (dx, dy), 0),
                    expected == b'1',
                    "direction={direction}, mask={mask}"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 16_384);
    }

    #[test]
    fn diagonal_solidtrans_matches_original_oracle() {
        // The original probe replaces `solid` with `solidtrans` on all four
        // squares and produces the same 16,384 outputs as this saved fixture.
        let oracle = include_str!("../../../tests/golden/collision_diagonal.txt");
        let mut checked = 0;
        for line in oracle.lines() {
            let (direction, bits) = line.split_once(':').expect("oracle line");
            let (dx, dy) = direction.split_once(',').expect("oracle direction");
            let dx: i32 = dx.parse().expect("dx");
            let dy: i32 = dy.parse().expect("dy");
            assert_eq!(bits.len(), 4096);
            for (mask, expected) in bits.bytes().enumerate() {
                let mut c = Collision::default();
                for (i, (x, y)) in [(0, 0), (dx, 0), (0, dy), (dx, dy)].into_iter().enumerate() {
                    let f = ((mask >> (3 * i)) & 7) as u16;
                    let flags = if f & BLOCK != 0 { f | SOLID_TRANS } else { f };
                    c.add(x, y, 0, flags);
                }
                assert_eq!(
                    !c.can_step((0, 0), (dx, dy), 0),
                    expected == b'1',
                    "direction={direction}, mask={mask}"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 16_384);
    }

    #[test]
    fn simultaneous_diagonal_crossing_does_not_cut_through_blocked_edge() {
        let mut c = Collision::default();
        // North edge of the southern neighbour blocks one of the four edges
        // checked by the original, even though the x-then-y path is open.
        c.add(0, 1, 0, EDGE_N);
        let mut player = Player {
            x: 0.95,
            y: 0.95,
            z: 0,
        };
        assert!(player.step(&c, 0.1, 0.1));
        assert!(player.x > 1.0);
        assert!(player.y < 1.0);
    }

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

/// Compile-time constants of `zombie.characters.IsoZombie` read from the class file (`node tools/classdump.mjs ../projectzomboid.jar zombie/characters/IsoZombie`).
/// These are facts about names and constants only: HOW the game combines them (vision falloff, fog, rain, darkness) is UNKNOWN and not yet used by the sim.
#[allow(dead_code)]
pub mod iso_zombie_constants {
    pub const VISION_RADIUS_MIN: f32 = 10.0;
    pub const VISION_RADIUS_MAX: f32 = 20.0;
    pub const VISION_FOG_PENALTY_MAX: f32 = 7.0;
    pub const VISION_RAIN_PENALTY_MAX: f32 = 2.5;
    pub const VISION_DARKNESS_PENALTY_MAX: f32 = 5.0;
    pub const HEARING_UNSEEN_OFFSET_MIN: i32 = 2;
    pub const HEARING_UNSEEN_OFFSET_HEAVY_RAIN: i32 = 5;
    pub const HEARING_UNSEEN_OFFSET_MAX: i32 = 10;
    pub const LUNGE_TIME: f32 = 180.0;
    pub const EAT_BODY_DIST: f32 = 1.0;
    pub const EAT_BODY_TIME: f32 = 3600.0;
    pub const CRAWLER_DAMAGE_RANGE: f32 = 1.5;
    pub const ALLOW_REPATH_DELAY_MAX: i32 = 120;
    // speedType: SPRINTER=1, FAST_SHAMBLER=2, SHAMBLER=3, RANDOM=4; hearing: PINPOINT=1, NORMAL=2, POOR=3, RANDOM=4, NORMAL_OR_POOR=5
}

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
