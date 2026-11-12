use super::iso_sprite_manager::SpriteRef;

/// zombie.iso.sprite.IsoSpriteGrid: the sprites of one multi-tile object facing, `width x height x levels`.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoSpriteGrid {
    sprites: Vec<Option<SpriteRef>>,
    width: i32,
    height: i32,
    levels: i32,
}

impl IsoSpriteGrid {
    pub fn new(width: i32, height: i32, levels: i32) -> Self {
        IsoSpriteGrid {
            sprites: vec![None; (width * height * levels) as usize],
            width,
            height,
            levels,
        }
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getAnchorSprite: first sprite on level 0.
    pub fn get_anchor_sprite(&self) -> Option<SpriteRef> {
        self.sprites
            .iter()
            .take((self.width * self.height) as usize)
            .flatten()
            .next()
            .copied()
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getSprite
    pub fn get_sprite(&self, x: i32, y: i32, z: i32) -> Option<SpriteRef> {
        self.get_sprite_index_xyz(x, y, z)
            .and_then(|i| self.sprites[i])
    }
    /// zombie.iso.sprite.IsoSpriteGrid.setSprite
    pub fn set_sprite(&mut self, x: i32, y: i32, z: i32, sprite: SpriteRef) {
        if let Some(i) = self.get_sprite_index_xyz(x, y, z) {
            self.sprites[i] = Some(sprite);
        }
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getSpriteIndex (IsoSprite): `None` is -1.
    pub fn get_sprite_index(&self, sprite: SpriteRef) -> Option<usize> {
        self.sprites.iter().position(|s| *s == Some(sprite))
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getSpriteGridPosX
    pub fn get_sprite_grid_pos_x(&self, sprite: SpriteRef) -> i32 {
        self.get_sprite_index(sprite)
            .map_or(-1, |i| (i as i32 % (self.width * self.height)) % self.width)
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getSpriteGridPosY
    pub fn get_sprite_grid_pos_y(&self, sprite: SpriteRef) -> i32 {
        self.get_sprite_index(sprite)
            .map_or(-1, |i| (i as i32 % (self.width * self.height)) / self.width)
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getSpriteGridPosZ
    pub fn get_sprite_grid_pos_z(&self, sprite: SpriteRef) -> i32 {
        self.get_sprite_index(sprite)
            .map_or(-1, |i| i as i32 / (self.width * self.height))
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getSpriteFromIndex
    pub fn get_sprite_from_index(&self, index: i32) -> Option<SpriteRef> {
        if index >= 0 {
            self.sprites.get(index as usize).copied().flatten()
        } else {
            None
        }
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getWidth
    pub fn get_width(&self) -> i32 {
        self.width
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getHeight
    pub fn get_height(&self) -> i32 {
        self.height
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getLevels
    pub fn get_levels(&self) -> i32 {
        self.levels
    }
    /// zombie.iso.sprite.IsoSpriteGrid.validate: every level must hold at least one sprite.
    pub fn validate(&self) -> bool {
        if self.levels == 0 {
            return false;
        }
        let per = (self.width * self.height) as usize;
        (0..self.levels as usize).all(|z| {
            self.sprites[z * per..(z + 1) * per]
                .iter()
                .any(Option::is_some)
        })
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getSpriteCount
    pub fn get_sprite_count(&self) -> usize {
        self.sprites.len()
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getSprites
    pub fn get_sprites(&self) -> &[Option<SpriteRef>] {
        &self.sprites
    }
    /// zombie.iso.sprite.IsoSpriteGrid.isValidXYZ
    pub fn is_valid_xyz(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height && z >= 0 && z < self.levels
    }
    /// zombie.iso.sprite.IsoSpriteGrid.getSpriteIndex (int, int, int): `None` is -1.
    pub fn get_sprite_index_xyz(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        self.is_valid_xyz(x, y, z)
            .then(|| (x + y * self.width + z * self.width * self.height) as usize)
    }
}
