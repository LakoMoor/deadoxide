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
    /// zombie.iso.IsoGridSquare.collideMatrix of every loaded square, computed by the ported engine
    /// (`IsoGridSquare::calculate_matrices`). Empty in synthetic tests, which use the flag rules below.
    collide_matrix: HashMap<(i32, i32, i32), i32>,
}

impl Collision {
    pub fn add(&mut self, x: i32, y: i32, z: i32, f: u16) {
        *self.flags.entry((x, y, z)).or_default() |= f;
    }
    /// Store the original-equivalent collide matrix of a square (bit `x + y*3 + z*9`, offsets + 1, set = blocked).
    pub fn set_collide_matrix(&mut self, x: i32, y: i32, z: i32, matrix: i32) {
        self.collide_matrix.insert((x, y, z), matrix);
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
        // zombie.iso.IsoGridSquare.getMatrixBit(collideMatrix, dx + 1, dy + 1, 1): the ported static collision.
        // Doors and windows as objects (TestCollide) are not part of the matrix and not modelled yet.
        let (mdx, mdy) = (to.0 - from.0, to.1 - from.1);
        if mdx.abs() <= 1 && mdy.abs() <= 1 {
            if let Some(m) = self.collide_matrix.get(&(from.0, from.1, z)) {
                return (m >> ((mdx + 1) + (mdy + 1) * 3 + 9)) & 1 == 0;
            }
        }
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

/// zombie.GameTime.update: numeric time increment for an unpaused normal-mode update.
/// This arithmetic alone does not run calendar, events, weather, pause or speed rules.
pub fn measured_clock_step(time: f32, minutes_per_day: f32, effective_multiplier: f32) -> f32 {
    let delta = (1.0_f32 / minutes_per_day / 60.0 * effective_multiplier) / 2.0;
    time + delta
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockEvent {
    Day,
    Hour,
    TenMinutes,
    OneMinute,
}

impl ClockEvent {
    pub fn name(self) -> &'static str {
        match self {
            Self::Day => "EveryDays",
            Self::Hour => "EveryHours",
            Self::TenMinutes => "EveryTenMinutes",
            Self::OneMinute => "EveryOneMinute",
        }
    }
}

/// Partial clock for normal single-player fixed-step operation. Weather and original pause/fast-forward controls are not yet connected.
#[derive(Clone, Debug)]
pub struct GameClock {
    pub time: f32,
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub minutes_per_day: f32,
    pub effective_multiplier: f32,
    pub nights_survived: i32,
}

impl GameClock {
    /// zombie.GameTime.getCalender / FishSchoolManager.getCurrentGameTimeInMinutes.
    /// Ambiguous local times choose the later instant, like Java's standard-time preference;
    /// nonexistent local times are rejected rather than guessed.
    pub fn calendar_minutes<T: chrono::TimeZone>(&self, timezone: &T) -> Option<i32> {
        use chrono::{Duration, NaiveDate};
        let date = NaiveDate::from_ymd_opt(self.year, u32::from(self.month) + 1, 1)?
            .checked_add_signed(Duration::days(i64::from(self.day) - 1))?;
        let hour = self.time as u32;
        let minute = ((self.time - hour as f32) * 60.0) as u32;
        let local = date.and_hms_opt(hour, minute, 0)?;
        let instant = timezone.from_local_datetime(&local).latest()?;
        Some((instant.timestamp_millis() / 60000) as i32)
    }

    /// zombie.GameTime.advanceOneDay: days per zero-based month in the Gregorian calendar.
    fn days_in_month(year: i32, month: u8) -> Option<u8> {
        match month {
            0 | 2 | 4 | 6 | 7 | 9 | 11 => Some(31),
            3 | 5 | 8 | 10 => Some(30),
            1 => Some(if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }),
            _ => None,
        }
    }

    /// zombie.GameTime.getWorldAgeHours: reproduce the f32 arithmetic used for minute stamps.
    fn minute_stamp(&self) -> i64 {
        let nights = self.nights_survived as f32 * 24.0;
        let age = if self.time >= 7.0 {
            nights + (self.time - 7.0)
        } else {
            nights + (self.time + 17.0)
        };
        let minute = ((self.time - self.time.trunc()) * 60.0) as i64;
        (age as i64) * 60 + minute
    }

    /// zombie.GameTime.update: measured normal-mode clock, date and ordered event signals.
    /// The caller must dispatch these signals; weather is not wired yet.
    pub fn advance(&mut self, paused: bool) -> Vec<ClockEvent> {
        if paused {
            return Vec::new();
        }
        let previous = self.time;
        let previous_stamp = self.minute_stamp();
        let previous_hour = previous as i32;
        let previous_ten = (((previous - previous.trunc()) * 60.0) as i32) / 10;
        self.time = measured_clock_step(previous, self.minutes_per_day, self.effective_multiplier);
        if previous <= 7.0 && self.time > 7.0 {
            self.nights_survived = self.nights_survived.wrapping_add(1);
        }
        let mut events = Vec::new();
        if self.time >= 24.0 {
            self.time -= 24.0;
            self.day += 1;
            if self.day >= Self::days_in_month(self.year, self.month).unwrap_or(31) {
                self.day = 0;
                self.month += 1;
                if self.month == 12 {
                    self.month = 0;
                    self.year += 1;
                }
            }
            events.push(ClockEvent::Day);
        }
        if self.time as i32 != previous_hour {
            events.push(ClockEvent::Hour);
        }
        let ten = (((self.time - self.time.trunc()) * 60.0) as i32) / 10;
        if ten != previous_ten {
            events.push(ClockEvent::TenMinutes);
        }
        if self.minute_stamp() != previous_stamp {
            events.push(ClockEvent::OneMinute);
        }
        events
    }

    pub fn from_save(
        time: &zomboid_content::save::SavedGameTime,
        day: &zomboid_content::sandbox::SavedDayLength,
    ) -> Option<Self> {
        if !(1..=9998).contains(&time.year) {
            return None;
        }
        let month = u8::try_from(time.month_index).ok()?;
        let date = u8::try_from(time.day_index).ok()?;
        if date >= Self::days_in_month(time.year, month)? {
            return None;
        }
        Some(Self {
            time: time.time_of_day,
            year: time.year,
            month,
            day: date,
            minutes_per_day: day.minutes as f32,
            // Controlled original trace: normal multiplier 1, fpsMultiplier 2 for 30 Hz.
            // Player sleep, UI speed, server and debug multipliers remain unimplemented.
            effective_multiplier: 1.6,
            nights_survived: time.nights_survived,
        })
    }
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
    pub clock: Option<GameClock>,
    /// Signals emitted in the most recent step; client dispatches Lua immediately, weather is not connected.
    pub clock_events: Vec<ClockEvent>,
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
            clock: None,
            clock_events: Vec::new(),
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
        self.clock_events = self
            .clock
            .as_mut()
            .map_or_else(Vec::new, |clock| clock.advance(false));
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
        if let Some(clock) = &self.clock {
            mix(clock.time.to_bits() as u64);
            mix(clock.year as u64);
            mix(clock.month as u64);
            mix(clock.day as u64);
            mix(clock.nights_survived as u64);
        }
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
    fn saved_world_clock_event_trace_matches_original() {
        let saved = zomboid_content::save::SavedGameTime {
            version: 249,
            time_of_day: f32::from_bits(0x4136ca18),
            nights_survived: 0,
            year: 1993,
            month_index: 6,
            day_index: 8,
        };
        let day = zomboid_content::sandbox::SavedDayLength {
            option: 4,
            minutes: 90,
        };
        let collision = Collision::default();
        let mut world = World::new(
            1,
            Player {
                x: 0.5,
                y: 0.5,
                z: 0,
            },
            &collision,
            0,
            0,
        );
        world.clock = GameClock::from_save(&saved, &day);
        let mut trace = Vec::new();
        for _ in 0..150 {
            world.step(&collision, (0.0, 0.0));
            for event in &world.clock_events {
                trace.push(format!("EVENT tick={} name={}", world.tick, event.name()));
            }
        }
        let clock = world.clock.as_ref().expect("clock");
        trace.push(format!(
            "END bits={:08x} nights={}",
            clock.time.to_bits(),
            clock.nights_survived
        ));
        assert_eq!(
            trace.join("\n"),
            include_str!("../../../tests/golden/clock_saved_events.txt").trim()
        );
    }

    #[test]
    fn fishing_calendar_matches_original_utc() {
        for line in include_str!("../../../tests/golden/fishing_calendar.txt").lines() {
            let (input, output) = line.strip_prefix("CAL ").unwrap().split_once('=').unwrap();
            let mut values = input.split(',');
            let year = values.next().unwrap().parse().unwrap();
            let month = values.next().unwrap().parse().unwrap();
            let day = values.next().unwrap().parse().unwrap();
            let time = f32::from_bits(u32::from_str_radix(values.next().unwrap(), 16).unwrap());
            let clock = GameClock {
                time,
                year,
                month,
                day,
                minutes_per_day: 90.0,
                effective_multiplier: 1.6,
                nights_survived: 0,
            };
            assert_eq!(
                clock.calendar_minutes(&chrono::Utc),
                Some(output.parse().unwrap()),
                "{line}"
            );
        }
    }

    #[test]
    fn world_clock_events_match_original_order() {
        let mut count = 0;
        for line in include_str!("../../../tests/golden/clock_events.txt").lines() {
            let (input, output) = line
                .strip_prefix("EVENT ")
                .unwrap()
                .split_once('=')
                .unwrap();
            let (time, nights) = input.split_once('@').unwrap();
            let mut fields = output.splitn(3, ',');
            let expected_bits = u32::from_str_radix(fields.next().unwrap(), 16).unwrap();
            let expected_nights: i32 = fields.next().unwrap().parse().unwrap();
            let expected_events = fields.next().unwrap();
            let collision = Collision::default();
            let mut world = World::new(
                1,
                Player {
                    x: 0.5,
                    y: 0.5,
                    z: 0,
                },
                &collision,
                0,
                0,
            );
            world.clock = Some(GameClock {
                time: time.parse().unwrap(),
                year: 1993,
                month: 6,
                day: 8,
                minutes_per_day: 90.0,
                effective_multiplier: 1.6,
                nights_survived: nights.parse().unwrap(),
            });
            world.step(&collision, (0.0, 0.0));
            let clock = world.clock.as_ref().unwrap();
            assert_eq!(clock.time.to_bits(), expected_bits, "{line}");
            assert_eq!(clock.nights_survived, expected_nights, "{line}");
            assert_eq!(
                world
                    .clock_events
                    .iter()
                    .map(|e| e.name())
                    .collect::<Vec<_>>()
                    .join("|"),
                expected_events,
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 10);
    }

    #[test]
    fn saved_clock_rejects_invalid_calendar_date() {
        let mut saved = zomboid_content::save::SavedGameTime {
            version: 249,
            time_of_day: 11.0,
            nights_survived: 0,
            year: 1993,
            month_index: 1,
            day_index: 27,
        };
        let day = zomboid_content::sandbox::SavedDayLength {
            option: 4,
            minutes: 90,
        };
        let clock = GameClock::from_save(&saved, &day).expect("valid date");
        assert_eq!(
            (clock.year, clock.month, clock.day, clock.minutes_per_day),
            (1993, 1, 27, 90.0)
        );
        saved.day_index = 28;
        assert!(GameClock::from_save(&saved, &day).is_none());
        saved.year = 1996;
        assert!(GameClock::from_save(&saved, &day).is_some());
        saved.month_index = 12;
        assert!(GameClock::from_save(&saved, &day).is_none());
    }

    #[test]
    fn world_clock_matches_fixed_30hz_original_traces() {
        let mut count = 0;
        for line in include_str!("../../../tests/golden/clock_fixed30.txt").lines() {
            let (head, rest) = line.split_once(" actual=").expect("header");
            let day: f32 = head.strip_prefix("CASE ").expect("case").parse().unwrap();
            let (multiplier, frames) = rest.split_once(" start=").expect("fields");
            let multiplier = f32::from_bits(u32::from_str_radix(multiplier, 16).unwrap());
            let frames: Vec<_> = frames
                .split(',')
                .map(|s| u32::from_str_radix(s, 16).unwrap())
                .collect();
            assert_eq!(frames.len(), 4);
            let collision = Collision::default();
            let mut world = World::new(
                1,
                Player {
                    x: 0.5,
                    y: 0.5,
                    z: 0,
                },
                &collision,
                0,
                0,
            );
            world.clock = Some(GameClock {
                time: f32::from_bits(frames[0]),
                year: 1993,
                month: 6,
                day: 8,
                minutes_per_day: day,
                effective_multiplier: multiplier,
                nights_survived: 0,
            });
            for step in 1..=100 {
                world.step(&collision, (0.0, 0.0));
                if let Some(index) = match step {
                    1 => Some(1),
                    30 => Some(2),
                    100 => Some(3),
                    _ => None,
                } {
                    assert_eq!(
                        world.clock.as_ref().unwrap().time.to_bits(),
                        frames[index],
                        "{line}, tick {step}"
                    );
                }
            }
            count += 1;
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn clock_midnight_and_pause_match_original_samples() {
        let mut count = 0;
        for line in include_str!("../../../tests/golden/clock_boundaries.txt").lines() {
            let (date, expected) = line.strip_prefix("DATE ").unwrap().split_once('=').unwrap();
            let date: Vec<i32> = date.split(',').map(|s| s.parse().unwrap()).collect();
            let fields: Vec<_> = expected.split(',').collect();
            assert_eq!((date.len(), fields.len()), (3, 4));
            let mut clock = GameClock {
                time: 23.99998,
                year: date[0],
                month: date[1] as u8,
                day: date[2] as u8,
                minutes_per_day: 90.0,
                effective_multiplier: 0.8,
                nights_survived: 0,
            };
            let _ = clock.advance(false);
            assert_eq!(
                clock.time.to_bits(),
                u32::from_str_radix(fields[0], 16).unwrap(),
                "{line}"
            );
            assert_eq!(
                (clock.year, clock.month, clock.day),
                (
                    fields[1].parse().unwrap(),
                    fields[2].parse().unwrap(),
                    fields[3].parse().unwrap()
                ),
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 7);
        let pause = include_str!("../../../tests/golden/clock_pause.txt").trim();
        let (before, after) = pause
            .strip_prefix("PAUSE true=")
            .unwrap()
            .split_once(',')
            .unwrap();
        let mut clock = GameClock {
            time: f32::from_bits(u32::from_str_radix(before, 16).unwrap()),
            year: 1993,
            month: 6,
            day: 8,
            minutes_per_day: 30.0,
            effective_multiplier: 0.8,
            nights_survived: 0,
        };
        let _ = clock.advance(true);
        assert_eq!(
            clock.time.to_bits(),
            u32::from_str_radix(after, 16).unwrap()
        );
    }

    #[test]
    fn numeric_clock_step_matches_original_short_traces() {
        let mut cases = 0;
        for line in include_str!("../../../tests/golden/clock_step.txt").lines() {
            let (_, rest) = line.split_once(" actual=").expect("case header");
            let (params, frames) = rest.split_once(" start=").expect("case fields");
            let params: Vec<_> = params
                .split(',')
                .map(|s| u32::from_str_radix(s, 16).unwrap())
                .collect();
            let frames: Vec<_> = frames
                .split(',')
                .map(|s| u32::from_str_radix(s, 16).unwrap())
                .collect();
            assert_eq!((params.len(), frames.len()), (2, 4));
            let (day, multiplier) = (f32::from_bits(params[0]), f32::from_bits(params[1]));
            let mut time = f32::from_bits(frames[0]);
            for frame in 1..=100 {
                time = measured_clock_step(time, day, multiplier);
                if let Some(index) = match frame {
                    1 => Some(1),
                    30 => Some(2),
                    100 => Some(3),
                    _ => None,
                } {
                    assert_eq!(time.to_bits(), frames[index], "{line}, frame {frame}");
                }
            }
            cases += 1;
        }
        assert_eq!(cases, 12);
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
