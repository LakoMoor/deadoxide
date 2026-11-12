use std::collections::BTreeMap;

use super::iso_grid_square::IsoGridSquare;

/// zombie.iso.IsoChunk: an 8x8 column of squares over all levels. So far the square storage the map loader fills.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct IsoChunk {
    pub wx: i32,
    pub wy: i32,
    min_level: i32,
    max_level: i32,
    /// level -> 64 squares (`x + y * 8`)
    squares: BTreeMap<i32, Vec<Option<Box<IsoGridSquare>>>>,
}

impl IsoChunk {
    pub fn new(wx: i32, wy: i32) -> Self {
        IsoChunk {
            wx,
            wy,
            ..Default::default()
        }
    }
    /// zombie.iso.IsoChunk.getMinLevel
    pub fn get_min_level(&self) -> i32 {
        self.min_level
    }
    /// zombie.iso.IsoChunk.getMaxLevel
    pub fn get_max_level(&self) -> i32 {
        self.max_level
    }
    /// zombie.iso.IsoChunk.getGridSquare: chunk-local x and y.
    pub fn get_grid_square(&self, x: i32, y: i32, z: i32) -> Option<&IsoGridSquare> {
        if !(0..8).contains(&x) || !(0..8).contains(&y) {
            return None;
        }
        self.squares.get(&z)?[(x + y * 8) as usize].as_deref()
    }
    pub fn get_grid_square_mut(&mut self, x: i32, y: i32, z: i32) -> Option<&mut IsoGridSquare> {
        if !(0..8).contains(&x) || !(0..8).contains(&y) {
            return None;
        }
        self.squares.get_mut(&z)?[(x + y * 8) as usize].as_deref_mut()
    }
    /// zombie.iso.IsoChunk.setSquare: also widens the chunk's level range to include `z`.
    pub fn set_square(&mut self, x: i32, y: i32, z: i32, square: Option<IsoGridSquare>) {
        self.min_level = self.min_level.min(z);
        self.max_level = self.max_level.max(z);
        for level in self.min_level..=self.max_level {
            self.squares.entry(level).or_insert_with(|| vec![None; 64]);
        }
        if let Some(l) = self.squares.get_mut(&z) {
            l[(x + y * 8) as usize] = square.map(Box::new);
        }
    }
}
