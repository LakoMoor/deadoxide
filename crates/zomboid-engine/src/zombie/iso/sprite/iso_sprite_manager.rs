use std::collections::HashMap;

use crate::zombie::iso::sprite_details::iso_flag_type::IsoFlagType;

use super::iso_sprite::IsoSprite;
use super::iso_sprite_grid::IsoSpriteGrid;

/// Handle to a sprite owned by an `IsoSpriteManager` (the original passes object references).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct SpriteRef(pub u32);
/// Handle to a sprite grid owned by an `IsoSpriteManager`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GridRef(pub u32);

/// zombie.iso.sprite.IsoSpriteManager: sprites by name and by numeric id. `IsoSpriteManager.instance` is a value here.
#[derive(Debug)]
pub struct IsoSpriteManager {
    sprites: Vec<IsoSprite>,
    grids: Vec<IsoSpriteGrid>,
    pub named_map: HashMap<String, SpriteRef>,
    pub int_map: HashMap<i32, SpriteRef>,
}

impl Default for IsoSpriteManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IsoSpriteManager {
    /// zombie.iso.sprite.IsoSpriteManager constructor: registers the invisible empty sprite under "".
    pub fn new() -> Self {
        let mut m = IsoSpriteManager {
            sprites: Vec::new(),
            grids: Vec::new(),
            named_map: HashMap::new(),
            int_map: HashMap::new(),
        };
        let mut sprite = IsoSprite {
            name: Some(String::new()),
            id: -1,
            ..IsoSprite::default()
        };
        sprite.properties.set(IsoFlagType::invisible);
        let r = m.alloc(sprite);
        m.named_map.insert(String::new(), r);
        m
    }

    fn alloc(&mut self, s: IsoSprite) -> SpriteRef {
        self.sprites.push(s);
        SpriteRef(self.sprites.len() as u32 - 1)
    }
    pub fn sprite(&self, r: SpriteRef) -> &IsoSprite {
        &self.sprites[r.0 as usize]
    }
    pub fn sprite_mut(&mut self, r: SpriteRef) -> &mut IsoSprite {
        &mut self.sprites[r.0 as usize]
    }
    pub fn add_grid(&mut self, g: IsoSpriteGrid) -> GridRef {
        self.grids.push(g);
        GridRef(self.grids.len() as u32 - 1)
    }
    pub fn grid(&self, r: GridRef) -> &IsoSpriteGrid {
        &self.grids[r.0 as usize]
    }

    /// zombie.iso.sprite.IsoSpriteManager.getSprite (int)
    pub fn get_sprite_by_id(&self, gid: i32) -> Option<SpriteRef> {
        self.int_map.get(&gid).copied()
    }

    /// zombie.iso.sprite.IsoSpriteManager.getSprite (String): an unknown name creates a sprite for that texture.
    pub fn get_sprite(&mut self, gid: &str) -> SpriteRef {
        match self.named_map.get(gid) {
            Some(r) => *r,
            None => self.add_sprite(gid),
        }
    }

    /// zombie.iso.sprite.IsoSpriteManager.AddSprite (String)
    pub fn add_sprite(&mut self, tex: &str) -> SpriteRef {
        // IsoSprite.LoadSingleTexture leaves `name` unset; only the texture is known.
        let r = self.alloc(IsoSprite {
            texture_name: Some(tex.to_string()),
            ..IsoSprite::default()
        });
        self.named_map.insert(tex.to_string(), r);
        r
    }

    /// zombie.iso.sprite.IsoSpriteManager.AddSprite (String, int): a repeated name gets a fresh sprite that takes
    /// over the name and reuses the first sprite's id.
    pub fn add_sprite_with_id(&mut self, tex: &str, mut id: i32) -> SpriteRef {
        let r = self.alloc(IsoSprite {
            texture_name: Some(tex.to_string()),
            ..IsoSprite::default()
        });
        if let Some(old) = self.named_map.get(tex) {
            id = self.sprite(*old).id;
        }
        self.named_map.insert(tex.to_string(), r);
        self.sprite_mut(r).id = id;
        self.int_map.insert(id, r);
        r
    }
}
