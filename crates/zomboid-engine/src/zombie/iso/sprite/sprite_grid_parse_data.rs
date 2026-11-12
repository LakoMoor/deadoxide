use std::collections::BTreeMap;

use super::iso_sprite_manager::SpriteRef;

/// zombie.iso.sprite.SpriteGridParseData.Level
#[derive(Default, Debug)]
pub struct Level {
    pub width: i32,
    pub height: i32,
    pub z: i32,
    pub xy_to_sprite: BTreeMap<String, SpriteRef>,
}
impl Level {
    /// zombie.iso.sprite.SpriteGridParseData.Level.isValid
    fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0 && !self.xy_to_sprite.is_empty()
    }
}

/// zombie.iso.sprite.SpriteGridParseData: scratch state while a sprite grid is assembled from tile properties.
#[derive(Default, Debug)]
pub struct SpriteGridParseData {
    pub levels: Vec<Level>,
    pub width: i32,
    pub height: i32,
}
impl SpriteGridParseData {
    /// zombie.iso.sprite.SpriteGridParseData.getOrCreateLevel: `None` where the original's `levels.get(z)` throws (negative z).
    pub fn get_or_create_level(&mut self, z: i32) -> Option<&mut Level> {
        let mut z1 = self.levels.len() as i32;
        while z1 <= z {
            self.levels.push(Level {
                z: z1,
                ..Level::default()
            });
            z1 += 1;
        }
        usize::try_from(z).ok().and_then(|z| self.levels.get_mut(z))
    }
    /// zombie.iso.sprite.SpriteGridParseData.isValid
    pub fn is_valid(&self) -> bool {
        self.width > 0
            && self.height > 0
            && !self.levels.is_empty()
            && self.levels.iter().all(Level::is_valid)
    }
    /// zombie.iso.sprite.SpriteGridParseData.clear
    pub fn clear(&mut self) {
        self.levels.clear();
        self.width = 0;
        self.height = 0;
    }
}
