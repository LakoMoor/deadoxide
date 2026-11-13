use super::sprite::iso_sprite_manager::{IsoSpriteManager, SpriteRef};
use super::sprite_details::iso_flag_type::IsoFlagType;
use super::sprite_details::iso_object_type::IsoObjectType;
use crate::zombie::core::tile_property_alias_map::TilePropertyAliasMap;

/// zombie.iso.objects.IsoDoor: the fields set when the map loader creates a door.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoDoor {
    pub open: bool,
    pub north: bool,
    pub locked: bool,
    pub locked_by_key: bool,
    pub closed_sprite: Option<SpriteRef>,
    pub open_sprite: Option<SpriteRef>,
}

/// zombie.iso.objects.IsoWindow: the fields set when the map loader creates a window.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoWindow {
    pub north: bool,
    pub open: bool,
    pub locked: bool,
    pub perma_locked: bool,
    pub smashed: bool,
    pub glass_removed: bool,
    pub closed_sprite: Option<SpriteRef>,
    pub open_sprite: Option<SpriteRef>,
    pub smashed_sprite: Option<SpriteRef>,
    pub glass_removed_sprite: Option<SpriteRef>,
}

/// zombie.iso.objects.IsoCurtain
#[derive(Clone, Debug, PartialEq)]
pub struct IsoCurtain {
    pub north: bool,
    pub open: bool,
    pub closed_sprite: Option<SpriteRef>,
    pub open_sprite: Option<SpriteRef>,
}

/// zombie.iso.objects.IsoTree
#[derive(Clone, Debug, PartialEq)]
pub struct IsoTree {
    pub size: i32,
    pub log_yield: i32,
    pub damage: i32,
}

/// The Java class of a world object. Subclass state that the loader does not touch is not modelled yet.
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectKind {
    IsoObject,
    IsoDoor(IsoDoor),
    IsoWindow(IsoWindow),
    IsoWindowFrame {
        north: bool,
    },
    IsoCurtain(IsoCurtain),
    IsoLightSwitch,
    IsoFireplace,
    IsoCombinationWasherDryer,
    IsoClothingDryer,
    IsoClothingWasher,
    IsoStove,
    /// zombie.iso.objects.IsoBarbecue
    IsoBarbecue {
        has_propane_tank: bool,
        fuel_amount_minutes: i32,
    },
    IsoJukebox,
    IsoRadio,
    IsoTelevision,
    IsoMannequin,
    IsoTree(IsoTree),
}

impl ObjectKind {
    /// `getClass().getSimpleName()` of the original object.
    pub fn class_name(&self) -> &'static str {
        match self {
            ObjectKind::IsoObject => "IsoObject",
            ObjectKind::IsoDoor(_) => "IsoDoor",
            ObjectKind::IsoWindow(_) => "IsoWindow",
            ObjectKind::IsoWindowFrame { .. } => "IsoWindowFrame",
            ObjectKind::IsoCurtain(_) => "IsoCurtain",
            ObjectKind::IsoLightSwitch => "IsoLightSwitch",
            ObjectKind::IsoFireplace => "IsoFireplace",
            ObjectKind::IsoCombinationWasherDryer => "IsoCombinationWasherDryer",
            ObjectKind::IsoClothingDryer => "IsoClothingDryer",
            ObjectKind::IsoClothingWasher => "IsoClothingWasher",
            ObjectKind::IsoStove => "IsoStove",
            ObjectKind::IsoBarbecue { .. } => "IsoBarbecue",
            ObjectKind::IsoJukebox => "IsoJukebox",
            ObjectKind::IsoRadio => "IsoRadio",
            ObjectKind::IsoTelevision => "IsoTelevision",
            ObjectKind::IsoMannequin => "IsoMannequin",
            ObjectKind::IsoTree(_) => "IsoTree",
        }
    }
}

/// zombie.iso.IsoObject: an object standing on a square. Owned by its `IsoGridSquare`.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoObject {
    pub kind: ObjectKind,
    pub sprite: Option<SpriteRef>,
    /// zombie.iso.IsoObject.attachedAnimSprite: overlay sprites drawn with this object (`None` until the first one).
    pub attached_anim_sprite: Option<Vec<SpriteRef>>,
    /// zombie.iso.IsoObject.tile (setTile): the tile name the object was created from.
    pub tile: Option<String>,
    pub render_y_offset: f32,
    pub tintr: f32,
    pub tintg: f32,
    pub tintb: f32,
    pub outline_on_mouseover: bool,
    /// zombie.iso.IsoObject.rerouteCollide is set (a window took over this wall's collision).
    pub reroute_collide: bool,
    /// The object is also in `IsoGridSquare.specialObjects`.
    pub special: bool,
}

impl IsoObject {
    pub fn new(kind: ObjectKind, sprite: Option<SpriteRef>) -> Self {
        IsoObject {
            kind,
            sprite,
            attached_anim_sprite: None,
            tile: None,
            render_y_offset: 0.0,
            tintr: 1.0,
            tintg: 1.0,
            tintb: 1.0,
            outline_on_mouseover: false,
            reroute_collide: false,
            special: false,
        }
    }
    /// zombie.iso.objects.IsoDoor.ToggleDoorSilent: flips a door between open and closed and shows the matching
    /// sprite. No effect on other objects. Barricades are not modelled. The caller recalculates the square afterwards.
    pub fn toggle_door_silent(&mut self) -> bool {
        let ObjectKind::IsoDoor(d) = &mut self.kind else {
            return false;
        };
        d.open = !d.open;
        self.sprite = if d.open {
            d.open_sprite
        } else {
            d.closed_sprite
        };
        true
    }

    /// zombie.iso.IsoObject.hasProperty (IsoFlagType)
    pub fn has_property(&self, sm: &IsoSpriteManager, flag: IsoFlagType) -> bool {
        self.sprite
            .is_some_and(|s| sm.sprite(s).properties.has(flag))
    }
    /// zombie.iso.IsoObject.hasProperty (IsoPropertyType)
    pub fn has_property_str(
        &self,
        sm: &IsoSpriteManager,
        alias: &TilePropertyAliasMap,
        name: &str,
    ) -> bool {
        self.sprite
            .is_some_and(|s| sm.sprite(s).properties.has_str(alias, name))
    }
    /// zombie.iso.IsoObject.getType
    pub fn get_type(&self, sm: &IsoSpriteManager) -> IsoObjectType {
        self.sprite
            .map_or(IsoObjectType::MAX, |s| sm.sprite(s).tile_type)
    }
    /// zombie.iso.IsoObject.isWallSE
    pub fn is_wall_se(&self, sm: &IsoSpriteManager) -> bool {
        self.has_property(sm, IsoFlagType::WallSE)
    }
    /// zombie.iso.IsoObject.isTableSurface
    pub fn is_table_surface(&self, sm: &IsoSpriteManager, alias: &TilePropertyAliasMap) -> bool {
        self.sprite
            .is_some_and(|s| sm.sprite(s).properties.is_table(alias))
    }
    /// zombie.iso.IsoObject.getSurfaceOffset
    pub fn get_surface_offset(&self, sm: &IsoSpriteManager, alias: &TilePropertyAliasMap) -> f32 {
        match self.sprite {
            Some(s) if self.is_table_surface(sm, alias) => {
                sm.sprite(s).properties.get_surface(alias) as f32
            }
            _ => 0.0,
        }
    }
    /// zombie.iso.IsoObject.attachedAnimSprite.add(IsoSpriteInstance.get(spr))
    pub fn attach(&mut self, spr: SpriteRef) {
        self.attached_anim_sprite
            .get_or_insert_with(|| Vec::with_capacity(4))
            .push(spr);
    }
}
