use super::iso_object::{IsoObject, ObjectKind};
use super::sprite::iso_sprite_manager::IsoSpriteManager;
use super::sprite_details::iso_flag_type::IsoFlagType;
use super::sprite_details::iso_object_type::IsoObjectType;
use crate::java;
use crate::zombie::core::properties::iso_property_type as pt;
use crate::zombie::core::tile_property_alias_map::TilePropertyAliasMap;

/// zombie.iso.IsoGridSquare: one square of the world and the objects on it, bottom to top. So far the state the map
/// loader fills. `specialObjects` of the original is the subset of `objects` flagged `special`, in list order.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoGridSquare {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub room_id: i64,
    pub objects: Vec<IsoObject>,
}

impl IsoGridSquare {
    /// zombie.iso.IsoGridSquare.getNew
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        IsoGridSquare {
            x,
            y,
            z,
            room_id: -1,
            objects: Vec::new(),
        }
    }

    /// zombie.iso.IsoGridSquare.placeWallAndDoorCheck: where `obj` goes so that doors stay above the wall or door frame
    /// they belong to. Returns `index` unchanged when no reordering is needed.
    pub fn place_wall_and_door_check(
        &self,
        sm: &IsoSpriteManager,
        obj: &IsoObject,
        index: i32,
    ) -> i32 {
        let mut needle_index: i32 = -1;
        let Some(os) = obj.sprite else { return index };
        let s = sm.sprite(os);
        let t = s.tile_type;
        let find_walls = t == IsoObjectType::doorN || t == IsoObjectType::doorW;
        let find_doors = !find_walls
            && (s.cut_w
                || s.cut_n
                || t == IsoObjectType::doorFrN
                || t == IsoObjectType::doorFrW
                || s.treat_as_wall_order);
        if find_doors || find_walls {
            for (i, other) in self.objects.iter().enumerate() {
                let Some(osr) = other.sprite else { continue };
                let o = sm.sprite(osr);
                let t = o.tile_type;
                if find_doors && (t == IsoObjectType::doorN || t == IsoObjectType::doorW) {
                    needle_index = i as i32;
                }
                if find_walls
                    && (t == IsoObjectType::doorFrN
                        || t == IsoObjectType::doorFrW
                        || o.cut_w
                        || o.cut_n
                        || o.treat_as_wall_order)
                {
                    needle_index = i as i32;
                }
            }
            if find_walls && needle_index > index {
                return needle_index + 1;
            }
            if find_doors && needle_index >= 0 && (needle_index < index || index < 0) {
                return needle_index;
            }
        }
        index
    }

    /// zombie.iso.IsoGridSquare.FixStackableObjects: swaps in "stacked" sprite variants and stacks table tops on tables
    /// by setting their render offset.
    pub fn fix_stackable_objects(&mut self, sm: &IsoSpriteManager, alias: &TilePropertyAliasMap) {
        let mut table: Option<usize> = None;
        for i in 0..self.objects.len() {
            let Some(mut spr) = self.objects[i].sprite else {
                continue;
            };
            let off = sm
                .sprite(spr)
                .properties
                .get_stack_replace_tile_offset(alias);
            if off != 0 {
                self.objects[i].sprite = sm.get_sprite_by_id(sm.sprite(spr).id.wrapping_add(off));
                match self.objects[i].sprite {
                    Some(s) => spr = s,
                    None => continue,
                }
            }
            let props = &sm.sprite(spr).properties;
            if props.is_table(alias) && !props.has_str(alias, pt::COUNTERTOP)
                || props.is_table_top(alias)
            {
                let offset = if props.is_surface_offset(alias) {
                    props.get_surface(alias) as f32
                } else {
                    0.0
                };
                self.objects[i].render_y_offset = match table {
                    Some(t) => {
                        self.objects[t].render_y_offset
                            + self.objects[t].get_surface_offset(sm, alias)
                            - offset
                    }
                    None => 0.0 - offset,
                };
            }
            if props.is_table(alias) {
                table = Some(i);
            }
        }
    }

    /// zombie.iso.IsoGridSquare.getFloor
    pub fn get_floor(&self, sm: &IsoSpriteManager) -> Option<usize> {
        self.objects
            .iter()
            .position(|o| o.has_property(sm, IsoFlagType::solidfloor))
    }
    /// zombie.iso.IsoGridSquare.getWall
    pub fn get_wall(&self, sm: &IsoSpriteManager, b_north: bool) -> Option<usize> {
        self.objects.iter().position(|o| {
            o.sprite.is_some_and(|s| {
                !o.is_wall_se(sm)
                    && (sm.sprite(s).cut_n && b_north || sm.sprite(s).cut_w && !b_north)
            })
        })
    }
    /// zombie.iso.IsoGridSquare.getWallSE
    pub fn get_wall_se(&self, sm: &IsoSpriteManager) -> Option<usize> {
        self.objects
            .iter()
            .position(|o| o.sprite.is_some() && o.is_wall_se(sm))
    }
    /// zombie.iso.IsoGridSquare.getWindow (GridSquareEdgeFacingDirection): the first special object that is a window on
    /// that edge (`north` = NORTH_SOUTH).
    pub fn get_window(&self, north: bool) -> Option<usize> {
        self.objects.iter().position(|o| {
            o.special && matches!(&o.kind, ObjectKind::IsoWindow(w) if w.north == north)
        })
    }
    /// zombie.iso.IsoGridSquare.getGarageDoor with zombie.iso.objects.IsoDoor.getGarageDoorIndex: `Err` is the
    /// NumberFormatException of a malformed GarageDoor property.
    pub fn get_garage_door(
        &self,
        sm: &IsoSpriteManager,
        alias: &TilePropertyAliasMap,
        b_north: bool,
    ) -> Result<Option<usize>, String> {
        for (n, obj) in self.objects.iter().enumerate() {
            let Some(s) = obj.sprite else { continue };
            let Some(v) = sm.sprite(s).properties.get(alias, pt::GARAGE_DOOR) else {
                continue;
            };
            let dd_index = java::parse_int(v).ok_or_else(|| {
                format!("java.lang.NumberFormatException: For input string: \"{v}\"")
            })?;
            if !(1..=6).contains(&dd_index) {
                continue;
            }
            let ObjectKind::IsoDoor(door) = &obj.kind else {
                continue;
            };
            let index = if !door.open {
                dd_index
            } else if dd_index >= 4 {
                dd_index - 3
            } else {
                -1
            };
            if index != -1 && b_north == door.north {
                return Ok(Some(n));
            }
        }
        Ok(None)
    }
    /// zombie.iso.IsoGridSquare.getCountertopAttachObject
    pub fn get_countertop_attach_object(
        &self,
        sm: &IsoSpriteManager,
        alias: &TilePropertyAliasMap,
    ) -> Option<usize> {
        self.objects
            .iter()
            .rposition(|o| o.has_property_str(sm, alias, pt::COUNTERTOP_ATTACH))
    }
}
