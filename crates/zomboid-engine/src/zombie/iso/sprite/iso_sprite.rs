use crate::zombie::core::properties::fascia_edge::FasciaEdge;
use crate::zombie::core::properties::property_container::PropertyContainer;
use crate::zombie::iso::sprite_details::iso_flag_type::IsoFlagType;
use crate::zombie::iso::sprite_details::iso_object_type::IsoObjectType;

use super::iso_sprite_manager::{GridRef, IsoSpriteManager, SpriteRef};

/// zombie.iso.sprite.IsoSprite.DEFAULT_SPRITE_ID
pub const DEFAULT_SPRITE_ID: i32 = 20000000;
/// zombie.iso.sprite.IsoSprite.RL_DEFAULT
pub const RL_DEFAULT: u8 = 0;
/// zombie.iso.sprite.IsoSprite.RL_FLOOR
pub const RL_FLOOR: u8 = 1;

/// zombie.iso.sprite.IsoSprite: the data side (definition and properties). Textures, animation and
/// rendering state of the original class are not here yet.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoSprite {
    pub firerequirement: i32,
    pub burnt_tile: Option<String>,
    pub force_ambient: bool,
    pub solidfloor: bool,
    pub can_be_removed: bool,
    pub attached_floor: bool,
    pub cut_w: bool,
    pub cut_n: bool,
    pub solid: bool,
    pub solid_trans: bool,
    pub invisible: bool,
    pub always_draw: bool,
    pub force_render: bool,
    pub move_with_wind: bool,
    pub is_bush: bool,
    pub render_layer: u8,
    pub wind_type: i32,
    pub properties: PropertyContainer,
    /// `None` only for sprites created without a texture name.
    pub name: Option<String>,
    pub tileset_name: Option<String>,
    pub tile_sheet_index: i32,
    pub id: i32,
    pub tile_type: IsoObjectType,
    pub parent_object_name: Option<String>,
    pub sprite_grid: Option<GridRef>,
    pub treat_as_wall_order: bool,
    pub fascia_edge: Option<FasciaEdge>,
    /// Name passed to `LoadSingleTexture`, the texture this sprite draws when it has no animation.
    pub texture_name: Option<String>,
}

impl Default for IsoSprite {
    fn default() -> Self {
        IsoSprite {
            firerequirement: 0,
            burnt_tile: None,
            force_ambient: false,
            solidfloor: false,
            can_be_removed: false,
            attached_floor: false,
            cut_w: false,
            cut_n: false,
            solid: false,
            solid_trans: false,
            invisible: false,
            always_draw: false,
            force_render: false,
            move_with_wind: false,
            is_bush: false,
            render_layer: RL_DEFAULT,
            wind_type: 1,
            properties: PropertyContainer::new(),
            name: None,
            tileset_name: None,
            tile_sheet_index: 0,
            id: DEFAULT_SPRITE_ID,
            tile_type: IsoObjectType::MAX,
            parent_object_name: None,
            sprite_grid: None,
            treat_as_wall_order: false,
            fascia_edge: None,
            texture_name: None,
        }
    }
}

impl IsoSprite {
    /// zombie.iso.sprite.IsoSprite.getProperties
    pub fn get_properties(&self) -> &PropertyContainer {
        &self.properties
    }
    /// zombie.iso.sprite.IsoSprite.getTileType
    pub fn get_tile_type(&self) -> IsoObjectType {
        self.tile_type
    }
    /// zombie.iso.sprite.IsoSprite.setTileType
    pub fn set_tile_type(&mut self, t: IsoObjectType) {
        self.tile_type = t;
    }
    /// zombie.iso.sprite.IsoSprite.is
    pub fn is(&self, flag: IsoFlagType) -> bool {
        self.properties.has(flag)
    }
    /// zombie.iso.sprite.IsoSprite.getName
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// zombie.iso.sprite.IsoSprite.getSprite (IsoSpriteManager, IsoSprite, int offset): the sprite `offset` tiles
/// further in the same sheet, looked up by name. `Err` where the original throws NumberFormatException.
pub fn get_sprite_offset(
    manager: &IsoSpriteManager,
    spr: SpriteRef,
    offset: i32,
) -> Result<Option<SpriteRef>, String> {
    let name = manager.sprite(spr).name.as_deref().unwrap_or("");
    let Some(pos) = name.rfind('_') else {
        return Ok(None);
    };
    // Java's split("_") drops trailing empty strings, so "a_" ends with "a"; rare enough to treat as the parse error it becomes.
    let last = crate::java::trim(&name[pos + 1..]);
    let id = crate::java::parse_int(last).ok_or_else(|| format!("For input string: \"{last}\""))?;
    Ok(manager
        .named_map
        .get(&format!("{}_{}", &name[..pos], id.wrapping_add(offset)))
        .copied())
}
