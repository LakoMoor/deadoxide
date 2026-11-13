use super::iso_directions::IsoDirections;
use super::iso_object::{IsoObject, ObjectKind};
use super::sprite::iso_sprite_manager::IsoSpriteManager;
use super::sprite_details::iso_flag_type::IsoFlagType;
use super::sprite_details::iso_object_type::IsoObjectType;
use crate::java;
use crate::zombie::core::properties::iso_property_type as pt;
use crate::zombie::core::properties::property_container::PropertyContainer;
use crate::zombie::core::tile_property_alias_map::TilePropertyAliasMap;

/// zombie.iso.IsoGridSquare: one square of the world and the objects on it, bottom to top. So far the state the map
/// loader fills. `specialObjects` of the original is the subset of `objects` flagged `special`, in list order.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoGridSquare {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub room_id: i64,
    /// zombie.iso.IsoGridSquare.room: index into `IsoMetaGrid::iso_rooms`.
    pub room: Option<usize>,
    /// zombie.iso.IsoGridSquare.roofHideBuilding: index into `IsoMetaGrid::iso_buildings`.
    pub roof_hide_building: Option<usize>,
    pub objects: Vec<IsoObject>,
    /// zombie.iso.IsoGridSquare.properties: the union of the objects' sprite properties, built by `recalc_properties`.
    pub properties: PropertyContainer,
    /// zombie.iso.IsoGridSquare.hasTypes: bit per `IsoObjectType` present.
    pub has_types: u64,
    pub has_tree: bool,
    /// zombie.iso.IsoGridSquare.haveRoof
    pub have_roof: bool,
    pub collide_matrix: i32,
    pub path_matrix: i32,
    pub vision_matrix: i32,
}

impl IsoGridSquare {
    /// zombie.iso.IsoGridSquare.getNew
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        IsoGridSquare {
            x,
            y,
            z,
            room_id: -1,
            room: None,
            roof_hide_building: None,
            properties: PropertyContainer::new(),
            has_types: 0,
            has_tree: false,
            have_roof: false,
            collide_matrix: 134217727,
            path_matrix: 134217727,
            vision_matrix: 0,
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

/// zombie.iso.IsoGridSquare.GetSquare: how the collision code looks up other squares.
pub trait GetSquare {
    fn get_grid_square(&self, x: i32, y: i32, z: i32) -> Option<&IsoGridSquare>;
}

use IsoFlagType as F;
use IsoObjectType as T;

impl IsoGridSquare {
    /// zombie.iso.IsoGridSquare.RecalcProperties: rebuilds the square's own property set from its objects, then
    /// resolves conflicts between them (a solid floor over water, a wall next to a hoppable fence, ...).
    /// Rain, burnt-out state, the stored fuel amount and zones are not modelled yet.
    pub fn recalc_properties(&mut self, sm: &IsoSpriteManager) {
        self.properties.clear();
        self.has_types = 0;
        self.has_tree = false;
        let (mut non_water_solid_trans, mut non_water_solid_floor, mut non_transparent_solid_floor) =
            (false, false, false);
        let (
            mut non_hoppable_collide_n,
            mut non_hoppable_collide_w,
            mut non_transparent_cut_n,
            mut non_transparent_cut_w,
        ) = (false, false, false, false);
        let mut force_render = false;
        for obj in &self.objects {
            let Some(spr) = obj.sprite else { continue };
            let sprite = sm.sprite(spr);
            let sp = &sprite.properties;
            if sp.has(F::blueprint) {
                continue;
            }
            if sprite.force_render {
                force_render = true;
            }
            if sprite.tile_type == T::tree {
                self.has_tree = true;
            }
            self.has_types |= 1u64 << sprite.tile_type.index();
            self.properties.add_properties(sp);
            if sp.has(F::water) {
                non_water_solid_floor = false;
            } else {
                non_water_solid_floor |= sp.has(F::solidfloor);
                non_water_solid_trans |= sp.has(F::solidtrans);
                non_transparent_solid_floor |=
                    sp.has(F::solidfloor) && !sp.has(F::transparentFloor);
            }
            non_hoppable_collide_n |= sp.has(F::collideN) && !sp.has(F::HoppableN);
            non_hoppable_collide_w |= sp.has(F::collideW) && !sp.has(F::HoppableW);
            non_transparent_cut_n |=
                sp.has(F::cutN) && !sp.has(F::transparentN) && !sp.has(F::WallSE);
            non_transparent_cut_w |=
                sp.has(F::cutW) && !sp.has(F::transparentW) && !sp.has(F::WallSE);
        }
        let p = &mut self.properties;
        if self.room_id == -1 && !self.have_roof {
            p.set(F::exterior);
        } else {
            p.unset(F::exterior);
        }
        if force_render {
            p.set(F::forceRender);
        }
        if !non_water_solid_trans && non_water_solid_floor && p.has(F::water) {
            p.unset(F::solidtrans);
        }
        if non_transparent_solid_floor && p.has(F::transparentFloor) {
            p.unset(F::transparentFloor);
        }
        if non_hoppable_collide_n && p.has(F::HoppableN) {
            p.unset(F::canPathN);
            p.unset(F::HoppableN);
        }
        if non_hoppable_collide_w && p.has(F::HoppableW) {
            p.unset(F::canPathW);
            p.unset(F::HoppableW);
        }
        if non_transparent_cut_n && p.has(F::transparentN) {
            p.unset(F::transparentN);
        }
        if non_transparent_cut_w && p.has(F::transparentW) {
            p.unset(F::transparentW);
        }
    }

    /// zombie.iso.IsoGridSquare.has (IsoFlagType)
    pub fn has(&self, flag: IsoFlagType) -> bool {
        self.properties.has(flag)
    }
    /// zombie.iso.IsoGridSquare.has (IsoObjectType)
    pub fn has_type(&self, t: IsoObjectType) -> bool {
        self.has_types & (1u64 << t.index()) != 0
    }
    /// zombie.iso.IsoGridSquare.hasFloorOverWater
    pub fn has_floor_over_water(&self) -> bool {
        self.has(F::water) && !self.has(F::solidtrans)
    }
    /// zombie.iso.IsoGridSquare.HasStairs
    pub fn has_stairs(&self) -> bool {
        [
            T::stairsTN,
            T::stairsMN,
            T::stairsBN,
            T::stairsTW,
            T::stairsMW,
            T::stairsBW,
        ]
        .iter()
        .any(|t| self.has_type(*t))
    }
    /// zombie.iso.IsoGridSquare.getStairs
    pub fn get_stairs(&self) -> IsoObjectType {
        [
            T::stairsTW,
            T::stairsTN,
            T::stairsMW,
            T::stairsMN,
            T::stairsBW,
            T::stairsBN,
        ]
        .into_iter()
        .find(|t| self.has_type(*t))
        .unwrap_or(T::MAX)
    }
    /// zombie.iso.IsoGridSquare.getSlopedSurfaceDirection
    pub fn get_sloped_surface_direction(
        &self,
        alias: &TilePropertyAliasMap,
    ) -> Option<IsoDirections> {
        self.properties.get_sloped_surface_direction(alias)
    }
    /// zombie.iso.IsoGridSquare.TreatAsSolidFloor (the original caches the answer between property changes).
    pub fn treat_as_solid_floor(&self, alias: &TilePropertyAliasMap) -> bool {
        self.has(F::solidfloor)
            || self.has_stairs()
            || self.get_sloped_surface_direction(alias).is_some()
    }
    /// zombie.iso.IsoGridSquare.getSlopedSurfaceHeight (IsoDirections): height (0..1) of the slope at that edge, -1 on the sides.
    pub fn get_sloped_surface_height(
        &self,
        alias: &TilePropertyAliasMap,
        edge: IsoDirections,
    ) -> f32 {
        match self.get_sloped_surface_direction(alias) {
            None => 0.0,
            Some(d) if d == edge => {
                self.properties.get_sloped_surface_height_max(alias) as f32 / 100.0
            }
            Some(d) if d.rot180() == edge => {
                self.properties.get_sloped_surface_height_min(alias) as f32 / 100.0
            }
            Some(_) => -1.0,
        }
    }
    /// zombie.iso.IsoGridSquare.hasSlopedSurfaceToLevelAbove
    pub fn has_sloped_surface_to_level_above(
        &self,
        alias: &TilePropertyAliasMap,
        dir: IsoDirections,
    ) -> bool {
        self.get_sloped_surface_direction(alias).is_some()
            && self.get_sloped_surface_height(alias, dir) == 1.0
    }
    /// zombie.iso.IsoGridSquare.getAdjacentSquare: the original reads the `nav` pointers; here the neighbour is looked up,
    /// which is the same once `doGridNav` has run for the area.
    fn adjacent<'a, G: GetSquare>(
        &self,
        g: &'a G,
        dir: IsoDirections,
    ) -> Option<&'a IsoGridSquare> {
        g.get_grid_square(self.x + dir.dx(), self.y + dir.dy(), self.z)
    }
    /// zombie.iso.IsoGridSquare.isSlopedSurfaceEdgeBlocked
    pub fn is_sloped_surface_edge_blocked<G: GetSquare>(
        &self,
        alias: &TilePropertyAliasMap,
        g: &G,
        edge: IsoDirections,
    ) -> bool {
        if self.get_sloped_surface_direction(alias).is_none() {
            return false;
        }
        match self.adjacent(g, edge) {
            None => true,
            Some(sq2) => {
                self.get_sloped_surface_height(alias, edge)
                    != sq2.get_sloped_surface_height(alias, edge.rot180())
            }
        }
    }
    /// zombie.iso.IsoGridSquare.isWall (GridSquareEdgeFacingDirection): `north` is NORTH_SOUTH.
    fn is_wall_facing(&self, north: bool) -> bool {
        if north {
            self.has(F::collideN) && !self.has(F::windowN)
        } else {
            self.has(F::collideW) && !self.has(F::windowW)
        }
    }
    /// zombie.iso.IsoGridSquare.isWallTo: a wall (not a window) on the edge between two orthogonally offset squares.
    /// As in the original, a diagonal pair tests both of its axes.
    pub fn is_wall_to<G: GetSquare>(&self, other: &IsoGridSquare, g: &G) -> bool {
        if std::ptr::eq(self, other) {
            return false;
        }
        // isEdgeElement: NORTH/WEST edges belong to the square itself, SOUTH/EAST to the neighbour on that side.
        let wall = |sq: &IsoGridSquare, edge: u8| match edge {
            0 => sq.is_wall_facing(true),
            1 => sq.is_wall_facing(false),
            2 => g
                .get_grid_square(sq.x, sq.y + 1, sq.z)
                .is_some_and(|s| s.is_wall_facing(true)),
            _ => g
                .get_grid_square(sq.x + 1, sq.y, sq.z)
                .is_some_and(|s| s.is_wall_facing(false)),
        };
        const NORTH: u8 = 0;
        const WEST: u8 = 1;
        const SOUTH: u8 = 2;
        const EAST: u8 = 3;
        self.x > other.x && (wall(self, WEST) || wall(other, EAST))
            || self.x < other.x && (wall(other, WEST) || wall(self, EAST))
            || self.y > other.y && (wall(self, NORTH) || wall(other, SOUTH))
            || self.y < other.y && (wall(other, NORTH) || wall(self, SOUTH))
    }

    /// zombie.iso.IsoGridSquare.CalculateCollide: is movement from this square to the adjacent `grid_square` blocked
    /// by the squares' static properties? `b_pathfind` asks for the path-finding variant (windows and hoppables pass).
    // The conditions keep the original's shape so they can be read side by side with the Java.
    #[allow(
        clippy::too_many_arguments,
        clippy::nonminimal_bool,
        clippy::collapsible_if,
        clippy::only_used_in_recursion
    )]
    pub fn calculate_collide<G: GetSquare>(
        &self,
        alias: &TilePropertyAliasMap,
        grid_square: Option<&IsoGridSquare>,
        b_vision: bool,
        b_pathfind: bool,
        b_ignore_solid_trans: bool,
        b_ignore_solid: bool,
        g: &G,
    ) -> bool {
        let Some(o) = grid_square else {
            return b_pathfind;
        };
        if !((!self.has(F::water) || self.has_floor_over_water())
            && (!o.has(F::water) || o.has_floor_over_water()))
        {
            return true;
        }
        let (test_w, test_n, test_e, test_s) =
            (o.x < self.x, o.y < self.y, o.x > self.x, o.y > self.y);
        if !b_ignore_solid && o.has(F::solid) {
            return if self.has_type(T::stairsTW)
                && !b_pathfind
                && o.x < self.x
                && o.y == self.y
                && o.z == self.z
            {
                false
            } else {
                !self.has_type(T::stairsTN)
                    || b_pathfind
                    || o.x != self.x
                    || o.y >= self.y
                    || o.z != self.z
            };
        }
        if !b_ignore_solid_trans && o.has(F::solidtrans) {
            if self.has_type(T::stairsTW)
                && !b_pathfind
                && o.x < self.x
                && o.y == self.y
                && o.z == self.z
            {
                return false;
            }
            if self.has_type(T::stairsTN)
                && !b_pathfind
                && o.x == self.x
                && o.y < self.y
                && o.z == self.z
            {
                return false;
            }
            let s = g.get_grid_square(o.x, o.y + 1, self.z);
            let e = g.get_grid_square(o.x + 1, o.y, self.z);
            let hop = |q: &IsoGridSquare| q.has(F::HoppableN) || q.has(F::HoppableW);
            let has_hoppable = hop(o) || s.is_some_and(hop) || e.is_some_and(hop);
            let has_window = o.has(F::windowW)
                || o.has(F::windowN)
                || o.has(F::WindowW)
                || o.has(F::WindowN)
                || s.is_some_and(|q| q.has(F::windowN) || q.has(F::WindowN))
                || e.is_some_and(|q| q.has(F::windowW) || q.has(F::WindowW));
            if !has_window && !has_hoppable {
                return true;
            }
        }
        if o.x != self.x && o.y != self.y && self.z != o.z && b_pathfind {
            return true;
        }
        if !(!b_pathfind || o.z >= self.z || self.treat_as_solid_floor(alias)) {
            return o.has_type(T::stairsTN) || o.has_type(T::stairsTW);
        }
        if b_pathfind && o.z == self.z {
            if o.x > self.x && o.y == self.y && o.has(F::windowW) {
                return false;
            }
            if o.y > self.y && o.x == self.x && o.has(F::windowN) {
                return false;
            }
            if o.x < self.x && o.y == self.y && self.has(F::windowW) {
                return false;
            }
            if o.y < self.y && o.x == self.x && self.has(F::windowN) {
                return false;
            }
        }
        if o.x > self.x && o.z < self.z && o.has_type(T::stairsTW) {
            return false;
        }
        if o.y > self.y && o.z < self.z && o.has_type(T::stairsTN) {
            return false;
        }
        let below_targ = g.get_grid_square(o.x, o.y, o.z - 1);
        let current_stairs = self.get_stairs();
        let target_stairs = o.get_stairs();
        let left_stairs = if self.x < o.x {
            current_stairs
        } else {
            target_stairs
        };
        let right_stairs = if self.x > o.x {
            current_stairs
        } else {
            target_stairs
        };
        let up_stairs = if self.y < o.y {
            current_stairs
        } else {
            target_stairs
        };
        let down_stairs = if self.y > o.y {
            current_stairs
        } else {
            target_stairs
        };
        let left_square = if self.x < o.x { self } else { o };
        let right_square = if self.x > o.x { self } else { o };
        let up_square = if self.y < o.y { self } else { o };
        let down_square = if self.y > o.y { self } else { o };
        let has_current_stairs = current_stairs != T::MAX;
        let has_target_stairs = target_stairs != T::MAX;
        let is_same_level = o.z == self.z;
        let is_diagonal = o.x != self.x && o.y != self.y;
        let is_at_top_stair = current_stairs == T::stairsTN || current_stairs == T::stairsTW;
        let is_targeting_top_stair = target_stairs == T::stairsTN || target_stairs == T::stairsTW;
        let is_top_stair_floor_transition =
            is_at_top_stair && !has_target_stairs || !has_current_stairs && is_targeting_top_stair;
        if is_same_level && self.is_wall_to(o, g) && !is_top_stair_floor_transition {
            return true;
        }
        if has_current_stairs
            && has_target_stairs
            && is_same_level
            && target_stairs == current_stairs
        {
            let is_adjacent = match current_stairs {
                T::stairsTN | T::stairsMN | T::stairsBN => self.x != o.x && self.y == o.y,
                T::stairsTW | T::stairsMW | T::stairsBW => self.x == o.x && self.y != o.y,
                _ => false,
            };
            if is_adjacent {
                return false;
            }
        }
        if is_same_level && is_diagonal && has_current_stairs && has_target_stairs {
            // isNorthFacingSequence / isWestFacingSequence
            if up_stairs == T::stairsTN && down_stairs == T::stairsMN
                || up_stairs == T::stairsMN && down_stairs == T::stairsBN
            {
                return false;
            }
            if left_stairs == T::stairsMW && right_stairs == T::stairsTW
                || left_stairs == T::stairsBW && right_stairs == T::stairsMW
            {
                return false;
            }
        }
        if is_diagonal && (has_current_stairs || has_target_stairs) {
            // hasConnectingBNStair / BWStair / TNStair / TWStair
            let adj = |sq: &IsoGridSquare, dir: IsoDirections, t: IsoObjectType| {
                sq.adjacent(g, dir).is_some_and(|a| a.has_type(t))
            };
            if is_same_level
                && up_stairs == T::stairsBN
                && adj(
                    up_square,
                    if left_stairs == up_stairs {
                        IsoDirections::E
                    } else {
                        IsoDirections::W
                    },
                    T::stairsBN,
                )
            {
                return false;
            }
            if is_same_level
                && right_stairs == T::stairsBW
                && adj(
                    right_square,
                    if left_stairs == right_stairs {
                        IsoDirections::S
                    } else {
                        IsoDirections::N
                    },
                    T::stairsBW,
                )
            {
                return false;
            }
            if down_stairs == T::stairsTN
                && down_square.z == up_square.z - 1
                && adj(
                    down_square,
                    if left_stairs == down_stairs {
                        IsoDirections::E
                    } else {
                        IsoDirections::W
                    },
                    T::stairsTN,
                )
            {
                return false;
            }
            if left_stairs == T::stairsTW
                && left_square.z == up_square.z - 1
                && adj(
                    left_square,
                    if left_stairs == up_stairs {
                        IsoDirections::S
                    } else {
                        IsoDirections::N
                    },
                    T::stairsTW,
                )
            {
                return false;
            }
        }
        let below_has = |t: IsoObjectType| below_targ.is_some_and(|b| b.has_type(t)) && !b_pathfind;
        if !(o.x == self.x || o.z != self.z || !o.has_type(T::stairsTN) || below_has(T::stairsTN)) {
            return true;
        }
        if !(o.y <= self.y
            || o.x != self.x
            || o.z != self.z
            || !o.has_type(T::stairsTN)
            || below_has(T::stairsTN))
        {
            return true;
        }
        if !(o.x <= self.x
            || o.y != self.y
            || o.z != self.z
            || !o.has_type(T::stairsTW)
            || below_has(T::stairsTW))
        {
            return true;
        }
        if !(o.y == self.y || o.z != self.z || !o.has_type(T::stairsTW) || below_has(T::stairsTW)) {
            return true;
        }
        let same = o.z == self.z;
        if o.x != self.x && same && o.has_type(T::stairsMN)
            || o.y != self.y && same && o.has_type(T::stairsMW)
            || o.x != self.x && same && o.has_type(T::stairsBN)
            || o.y != self.y && same && o.has_type(T::stairsBW)
        {
            return true;
        }
        if o.x != self.x && same && self.has_type(T::stairsTN)
            || o.y != self.y && same && self.has_type(T::stairsTW)
            || o.x != self.x && same && self.has_type(T::stairsMN)
            || o.y != self.y && same && self.has_type(T::stairsMW)
            || o.x != self.x && same && self.has_type(T::stairsBN)
            || o.y != self.y && same && self.has_type(T::stairsBW)
        {
            return true;
        }
        if o.y < self.y && o.x == self.x && o.z > self.z && self.has_type(T::stairsTN)
            || o.x < self.x && o.y == self.y && o.z > self.z && self.has_type(T::stairsTW)
            || o.y > self.y && o.x == self.x && o.z < self.z && o.has_type(T::stairsTN)
            || o.x > self.x && o.y == self.y && o.z < self.z && o.has_type(T::stairsTW)
        {
            return false;
        }
        use IsoDirections::{E, N, S, W};
        if same {
            for (dx, dy, d) in [(0, -1, N), (0, 1, S), (-1, 0, W), (1, 0, E)] {
                if o.x == self.x + dx
                    && o.y == self.y + dy
                    && !self.has_sloped_surface_to_level_above(alias, d)
                    && (self.is_sloped_surface_edge_blocked(alias, g, d)
                        || o.is_sloped_surface_edge_blocked(alias, g, d.rot180()))
                {
                    return true;
                }
            }
        }
        if o.z > self.z {
            if o.y < self.y && o.x == self.x && self.has_sloped_surface_to_level_above(alias, N)
                || o.y > self.y && o.x == self.x && self.has_sloped_surface_to_level_above(alias, S)
                || o.x < self.x && o.y == self.y && self.has_sloped_surface_to_level_above(alias, W)
                || o.x > self.x && o.y == self.y && self.has_sloped_surface_to_level_above(alias, E)
            {
                return false;
            }
        }
        if o.z < self.z {
            if o.y > self.y && o.x == self.x && o.has_sloped_surface_to_level_above(alias, N)
                || o.y < self.y && o.x == self.x && o.has_sloped_surface_to_level_above(alias, S)
                || o.x > self.x && o.y == self.y && o.has_sloped_surface_to_level_above(alias, W)
                || o.x < self.x && o.y == self.y && o.has_sloped_surface_to_level_above(alias, E)
            {
                return false;
            }
        }
        if same && !o.treat_as_solid_floor(alias) && b_pathfind {
            return true;
        }
        if same
            && !o.treat_as_solid_floor(alias)
            && o.z > 0
            && g.get_grid_square(o.x, o.y, o.z - 1).is_none()
        {
            return true;
        }
        if self.z != o.z {
            return o.z >= self.z
                || o.x != self.x
                || o.y != self.y
                || self.treat_as_solid_floor(alias);
        }
        let mut col_n = test_n && self.has(F::collideN);
        let mut col_w = test_w && self.has(F::collideW);
        let mut col_s = test_s && o.has(F::collideN);
        let mut col_e = test_e && o.has(F::collideW);
        if b_pathfind {
            col_n &= !self.has(F::canPathN);
            col_w &= !self.has(F::canPathW);
            col_s &= !o.has(F::canPathN);
            col_e &= !o.has(F::canPathW);
        }
        if col_w && self.has_type(T::stairsTW) && !b_pathfind {
            col_w = false;
        }
        if col_n && self.has_type(T::stairsTN) && !b_pathfind {
            col_n = false;
        }
        if col_n || col_w || col_s || col_e {
            return true;
        }
        if o.x != self.x && o.y != self.y {
            let between_a = g.get_grid_square(self.x, o.y, self.z);
            let between_b = g.get_grid_square(o.x, self.y, self.z);
            let same_sq = |a: Option<&IsoGridSquare>, b: &IsoGridSquare| {
                a.is_some_and(|a| std::ptr::eq(a, b))
            };
            let both_same = match (between_a, between_b) {
                (Some(a), Some(b)) => std::ptr::eq(a, b),
                (None, None) => true,
                _ => false,
            };
            if std::ptr::eq(o, self)
                || both_same
                || same_sq(between_a, self)
                || same_sq(between_b, self)
                || same_sq(between_a, o)
                || same_sq(between_b, o)
            {
                return true;
            }
            if let (Some(a), Some(b)) = (between_a, between_b) {
                if o.x == self.x + 1 && o.y == self.y + 1 && a.has(F::windowN) && b.has(F::windowW)
                {
                    return true;
                }
                if o.x == self.x - 1 && o.y == self.y - 1 && a.has(F::windowW) && b.has(F::windowN)
                {
                    return true;
                }
            }
            if self.calculate_collide(
                alias,
                between_a,
                b_vision,
                b_pathfind,
                b_ignore_solid_trans,
                false,
                g,
            ) || self.calculate_collide(
                alias,
                between_b,
                b_vision,
                b_pathfind,
                b_ignore_solid_trans,
                false,
                g,
            ) || o.calculate_collide(
                alias,
                between_a,
                b_vision,
                b_pathfind,
                b_ignore_solid_trans,
                false,
                g,
            ) || o.calculate_collide(
                alias,
                between_b,
                b_vision,
                b_pathfind,
                b_ignore_solid_trans,
                false,
                g,
            ) {
                return true;
            }
        }
        false
    }

    /// zombie.iso.IsoGridSquare.CalculateVisionBlocked: is the line of sight from this square to an adjacent one blocked
    /// by walls, floors or sight-blocking objects?
    pub fn calculate_vision_blocked<G: GetSquare>(
        &self,
        grid_square: Option<&IsoGridSquare>,
        g: &G,
    ) -> bool {
        let Some(o) = grid_square else { return false };
        if (o.x - self.x).abs() > 1 || (o.y - self.y).abs() > 1 {
            return true;
        }
        let (test_w, test_n, test_e, test_s) =
            (o.x < self.x, o.y < self.y, o.x > self.x, o.y > self.y);
        if o.has(F::trans) || self.has(F::trans) {
            return false;
        }
        let opaque_floor = |q: &IsoGridSquare| q.has(F::solidfloor) && !q.has(F::transparentFloor);
        if self.z != o.z {
            if o.z > self.z {
                if opaque_floor(o) || self.has(F::noStart) {
                    return true;
                }
                match g.get_grid_square(self.x, self.y, o.z) {
                    None => return false,
                    Some(sq) if opaque_floor(sq) => return true,
                    _ => {}
                }
            } else {
                if opaque_floor(self) || self.has(F::noStart) {
                    return true;
                }
                match g.get_grid_square(o.x, o.y, self.z) {
                    None => return false,
                    Some(sq) if opaque_floor(sq) => return true,
                    _ => {}
                }
            }
        }
        let col_n =
            test_n && self.has(F::collideN) && !self.has(F::transparentN) && !self.has(F::doorN);
        let col_w =
            test_w && self.has(F::collideW) && !self.has(F::transparentW) && !self.has(F::doorW);
        let col_s = test_s && o.has(F::collideN) && !o.has(F::transparentN) && !o.has(F::doorN);
        let col_e = test_e && o.has(F::collideW) && !o.has(F::transparentW) && !o.has(F::doorW);
        if col_n || col_w || col_s || col_e {
            return true;
        }
        if o.has(F::solid) || o.has(F::blocksight) {
            return true;
        }
        if o.x != self.x && o.y != self.y {
            let between_a = g.get_grid_square(self.x, o.y, self.z);
            let between_b = g.get_grid_square(o.x, self.y, self.z);
            if self.calculate_vision_blocked(between_a, g)
                || self.calculate_vision_blocked(between_b, g)
                || o.calculate_vision_blocked(between_a, g)
                || o.calculate_vision_blocked(between_b, g)
            {
                return true;
            }
        }
        false
    }

    /// The three 3x3x3 neighbour bit matrices as `RecalcAllWithNeighbours` leaves them once every square of an area has
    /// been processed: bit `x + y*3 + z*9` (offsets + 1) of (collide, path, vision). Missing neighbours and the centre
    /// keep the initial bits (blocked for movement, clear for vision).
    pub fn calculate_matrices<G: GetSquare>(
        &self,
        alias: &TilePropertyAliasMap,
        g: &G,
    ) -> (i32, i32, i32) {
        let (mut c, mut p, mut v) = (134217727i32, 134217727i32, 0i32);
        for lz in -1..=1 {
            for ly in -1..=1 {
                for lx in -1..=1 {
                    if lx == 0 && ly == 0 && lz == 0 {
                        continue;
                    }
                    let Some(sq) = g.get_grid_square(self.x + lx, self.y + ly, self.z + lz) else {
                        continue;
                    };
                    let bit = 1i32 << ((1 + lx) + (1 + ly) * 3 + (1 + lz) * 9);
                    let set = |m: &mut i32, b: bool| if b { *m |= bit } else { *m &= !bit };
                    set(
                        &mut c,
                        self.calculate_collide(alias, Some(sq), false, false, false, false, g),
                    );
                    set(
                        &mut p,
                        self.calculate_collide(alias, Some(sq), false, true, false, false, g),
                    );
                    set(&mut v, self.calculate_vision_blocked(Some(sq), g));
                }
            }
        }
        (c, p, v)
    }
}

/// A world object found by a square lookup: the square's coordinates and the index in its `objects`.
pub type ObjectAt = (i32, i32, i32, usize);

impl IsoGridSquare {
    fn at(&self, i: usize) -> ObjectAt {
        (self.x, self.y, self.z, i)
    }

    /// zombie.iso.IsoGridSquare.isEdgeElementTo with isEdgeElement: does `pred(square, north_edge)` hold on the edge
    /// shared with `other`? NORTH/WEST edges belong to a square itself, SOUTH/EAST to its neighbour on that side.
    fn is_edge_element_to<G: GetSquare>(
        &self,
        other: &IsoGridSquare,
        g: &G,
        pred: &dyn Fn(&IsoGridSquare, bool) -> bool,
    ) -> bool {
        if std::ptr::eq(self, other) {
            return false;
        }
        let north = |sq: &IsoGridSquare| pred(sq, true);
        let west = |sq: &IsoGridSquare| pred(sq, false);
        let south = |sq: &IsoGridSquare| {
            g.get_grid_square(sq.x, sq.y + 1, sq.z)
                .is_some_and(|s| pred(s, true))
        };
        let east = |sq: &IsoGridSquare| {
            g.get_grid_square(sq.x + 1, sq.y, sq.z)
                .is_some_and(|s| pred(s, false))
        };
        self.x > other.x && (west(self) || east(other))
            || self.x < other.x && (west(other) || east(self))
            || self.y > other.y && (north(self) || south(other))
            || self.y < other.y && (north(other) || south(self))
    }

    /// zombie.iso.IsoGridSquare.hasBlockedWindow: a window on that edge that is neither open nor smashed.
    /// Barricades are not modelled yet.
    fn has_blocked_window(&self, north: bool) -> bool {
        self.objects.iter().any(|o| matches!(&o.kind, ObjectKind::IsoWindow(w) if w.north == north && !w.smashed && !w.open))
    }
    /// zombie.iso.IsoGridSquare.hasBlockedDoor: a closed door on that edge.
    fn has_blocked_door(&self, north: bool) -> bool {
        self.objects
            .iter()
            .any(|o| matches!(&o.kind, ObjectKind::IsoDoor(d) if d.north == north && !d.open))
    }

    /// zombie.iso.IsoGridSquare.isBlockedTo: wall, closed window, closed door or the top of a staircase between the squares.
    pub fn is_blocked_to<G: GetSquare>(&self, other: &IsoGridSquare, g: &G) -> bool {
        self.is_wall_to(other, g)
            || self.is_edge_element_to(other, g, &|s, n| s.has_blocked_window(n))
            || self.is_edge_element_to(other, g, &|s, n| s.has_blocked_door(n))
            || (self.x > other.x && self.has_type(T::stairsTW))
            || (self.y > other.y && self.has_type(T::stairsTN))
    }

    /// zombie.iso.IsoGridSquare.getSpecialWall: the window, closed door or window frame standing on this square's
    /// north (`north`) or west edge. Player-built objects (IsoThumpable) are not modelled yet.
    pub fn get_special_wall(&self, north: bool) -> Option<usize> {
        for (i, o) in self.objects.iter().enumerate().rev() {
            if !o.special {
                continue;
            }
            match &o.kind {
                ObjectKind::IsoWindow(w) if w.north == north => return Some(i),
                ObjectKind::IsoDoor(d) if d.north == north && !d.open => return Some(i),
                _ => {}
            }
        }
        if self.has(if north { F::WindowN } else { F::WindowW }) {
            return self.objects.iter().position(
                |o| matches!(o.kind, ObjectKind::IsoWindowFrame { north: n } if n == north),
            );
        }
        None
    }

    /// zombie.iso.IsoGridSquare.testCollideSpecialObjects (IsoGridSquare): the door, window or window frame that stops
    /// a move from this square to the adjacent `next`, if any. `getSpecialSolid` (player-built and moved furniture) is
    /// always empty in a freshly loaded world and not modelled.
    pub fn test_collide_special_objects<G: GetSquare>(
        &self,
        alias: &TilePropertyAliasMap,
        next: Option<&IsoGridSquare>,
        g: &G,
    ) -> Option<ObjectAt> {
        let next = next?;
        if std::ptr::eq(next, self) {
            return None;
        }
        let wall = |sq: &IsoGridSquare, north: bool| sq.get_special_wall(north).map(|i| sq.at(i));
        let get = |x: i32, y: i32| g.get_grid_square(x, y, self.z);
        let (dx, dy) = ((next.x - self.x).signum(), (next.y - self.y).signum());
        match (dx, dy) {
            (-1, 0) => {
                if next.z == self.z
                    && (self.has_type(T::stairsTW)
                        || self.has_sloped_surface_to_level_above(alias, IsoDirections::W))
                {
                    return None;
                }
                wall(self, false)
            }
            (0, -1) => {
                if next.z == self.z
                    && (self.has_type(T::stairsTN)
                        || self.has_sloped_surface_to_level_above(alias, IsoDirections::N))
                {
                    return None;
                }
                wall(self, true)
            }
            (1, 0) => wall(next, false),
            (0, 1) => wall(next, true),
            (-1, -1) => {
                if let Some(o) = wall(self, true).or_else(|| wall(self, false)) {
                    return Some(o);
                }
                if let Some(a) = get(self.x, self.y - 1).filter(|a| !self.is_blocked_to(a, g)) {
                    if let Some(o) = wall(a, false) {
                        return Some(o);
                    }
                }
                if let Some(b) = get(self.x - 1, self.y).filter(|b| !self.is_blocked_to(b, g)) {
                    if let Some(o) = wall(b, true) {
                        return Some(o);
                    }
                }
                None
            }
            (1, -1) => {
                if let Some(o) = wall(self, true) {
                    return Some(o);
                }
                let a = get(self.x, self.y - 1);
                let b = get(self.x + 1, self.y);
                if let Some(b) = b {
                    if let Some(o) = wall(b, false) {
                        return Some(o);
                    }
                    if !self.is_blocked_to(b, g) {
                        if let Some(o) = wall(b, true) {
                            return Some(o);
                        }
                    }
                }
                match (a, b) {
                    (Some(a), Some(b))
                        if !self.is_blocked_to(a, g) && !self.is_blocked_to(b, g) =>
                    {
                        wall(next, false)
                    }
                    _ => None,
                }
            }
            (1, 1) => {
                let a = get(self.x, self.y + 1);
                let b = get(self.x + 1, self.y);
                if let Some(o) = a.and_then(|a| wall(a, true)) {
                    return Some(o);
                }
                if let Some(o) = b.and_then(|b| wall(b, false)) {
                    return Some(o);
                }
                match (a, b) {
                    (Some(a), Some(b))
                        if !self.is_blocked_to(a, g) && !self.is_blocked_to(b, g) =>
                    {
                        wall(next, false).or_else(|| wall(next, true))
                    }
                    _ => None,
                }
            }
            (-1, 1) => {
                if let Some(o) = wall(self, false) {
                    return Some(o);
                }
                let a = get(self.x, self.y + 1);
                let b = get(self.x - 1, self.y);
                if let Some(o) = a.and_then(|a| wall(a, true)) {
                    return Some(o);
                }
                match (a, b) {
                    (Some(a), Some(b))
                        if !self.is_blocked_to(a, g) && !self.is_blocked_to(b, g) =>
                    {
                        wall(next, true)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// zombie.iso.LosUtil.TestResults
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TestResults {
    Clear,
    ClearThroughOpenDoor,
    ClearThroughWindow,
    Blocked,
    ClearThroughClosedDoor,
}

/// zombie.iso.IsoObject.VisionResult
#[derive(Clone, Copy, PartialEq, Eq)]
enum VisionResult {
    NoEffect,
    Blocked,
    Unblocked,
}

impl IsoGridSquare {
    /// zombie.iso.IsoGridSquare.HasElevatedFloor
    pub fn has_elevated_floor(&self) -> bool {
        [T::stairsTN, T::stairsMN, T::stairsTW, T::stairsMW]
            .iter()
            .any(|t| self.has_type(*t))
    }

    /// `TestVision` of doors, windows and curtains (zombie.iso.objects.IsoDoor/IsoWindow/IsoCurtain.TestVision) for
    /// the object `o` standing on `owner`, looking from `from` to `to`. Door curtains are not modelled.
    fn object_test_vision(
        sm: &IsoSpriteManager,
        alias: &TilePropertyAliasMap,
        o: &IsoObject,
        owner: &IsoGridSquare,
        from: &IsoGridSquare,
        to: &IsoGridSquare,
    ) -> VisionResult {
        if to.z != from.z {
            return VisionResult::NoEffect;
        }
        let at_owner = std::ptr::eq(from, owner);
        // The edge test shared by all three: from the owner square towards north/west, or into it from south/east.
        let crosses = |north: bool| {
            if at_owner {
                north && to.y < from.y || !north && to.x < from.x
            } else {
                north && to.y > from.y || !north && to.x > from.x
            }
        };
        match &o.kind {
            ObjectKind::IsoDoor(d) => {
                let trans = d.open
                    || o.sprite
                        .is_some_and(|s| sm.sprite(s).properties.has_str(alias, pt::DOOR_TRANS));
                // An open door stands along the other edge.
                let north = if d.open { !d.north } else { d.north };
                if crosses(north) {
                    if trans {
                        VisionResult::Unblocked
                    } else {
                        VisionResult::Blocked
                    }
                } else {
                    VisionResult::NoEffect
                }
            }
            ObjectKind::IsoWindow(w) => {
                if crosses(w.north) {
                    VisionResult::Unblocked
                } else {
                    VisionResult::NoEffect
                }
            }
            ObjectKind::IsoCurtain(c) => {
                let ty = o.get_type(sm);
                let own_side = at_owner && (ty == T::curtainW || ty == T::curtainN)
                    || !at_owner && (ty == T::curtainE || ty == T::curtainS);
                let hit = if own_side {
                    c.north && to.y < from.y || !c.north && to.x < from.x
                } else {
                    c.north && to.y > from.y || !c.north && to.x > from.x
                };
                if hit && !c.open {
                    VisionResult::Blocked
                } else {
                    VisionResult::NoEffect
                }
            }
            _ => VisionResult::NoEffect,
        }
    }

    /// zombie.iso.IsoGridSquare.DoDiagnalCheck
    #[allow(clippy::too_many_arguments)]
    fn do_diagnal_check<G: GetSquare>(
        &self,
        sm: &IsoSpriteManager,
        alias: &TilePropertyAliasMap,
        g: &G,
        x: i32,
        y: i32,
        z: i32,
        ignore_doors: bool,
        server: bool,
    ) -> TestResults {
        let res = self.test_vision_adjacent(sm, alias, g, x, 0, z, false, ignore_doors, server);
        if res == TestResults::Blocked {
            return TestResults::Blocked;
        }
        let res2 = self.test_vision_adjacent(sm, alias, g, 0, y, z, false, ignore_doors, server);
        if res2 == TestResults::Blocked {
            return TestResults::Blocked;
        }
        if res != TestResults::ClearThroughWindow && res2 != TestResults::ClearThroughWindow {
            self.test_vision_adjacent(sm, alias, g, x, y, z, false, ignore_doors, server)
        } else {
            TestResults::ClearThroughWindow
        }
    }

    /// zombie.iso.IsoGridSquare.testVisionAdjacent: can one see from this square into the neighbour at offset
    /// (x, y, z)? Uses the stored `vision_matrix` plus doors, windows and curtains on both squares. `server` is
    /// `GameServer.server` (a client also checks diagonals from the far side).
    #[allow(clippy::too_many_arguments)]
    pub fn test_vision_adjacent<G: GetSquare>(
        &self,
        sm: &IsoSpriteManager,
        alias: &TilePropertyAliasMap,
        g: &G,
        x: i32,
        y: i32,
        z: i32,
        special_diag: bool,
        ignore_doors: bool,
        server: bool,
    ) -> TestResults {
        if !(-1..=1).contains(&x) || !(-1..=1).contains(&y) || !(-1..=1).contains(&z) {
            return TestResults::Blocked;
        }
        if z == 1 && (x != 0 || y != 0) && self.has_elevated_floor() {
            if let Some(sq) = g.get_grid_square(self.x, self.y, self.z + z) {
                return sq.test_vision_adjacent(
                    sm,
                    alias,
                    g,
                    x,
                    y,
                    0,
                    special_diag,
                    ignore_doors,
                    server,
                );
            }
        }
        if z == -1
            && (x != 0 || y != 0)
            && g.get_grid_square(self.x + x, self.y + y, self.z + z)
                .is_some_and(|s| s.has_elevated_floor())
        {
            return self.test_vision_adjacent(
                sm,
                alias,
                g,
                x,
                y,
                0,
                special_diag,
                ignore_doors,
                server,
            );
        }
        if x != 0 && y != 0 && special_diag {
            let mut test = self.do_diagnal_check(sm, alias, g, x, y, z, ignore_doors, server);
            if !server && test != TestResults::Blocked {
                if let Some(sq) = g.get_grid_square(self.x + x, self.y + y, self.z + z) {
                    test = sq.do_diagnal_check(sm, alias, g, -x, -y, -z, ignore_doors, server);
                }
            }
            return test;
        }
        let sq = g.get_grid_square(self.x + x, self.y + y, self.z + z);
        let mut ret = TestResults::Clear;
        match sq {
            Some(sq) if sq.z == self.z => {
                for owner in [self, sq] {
                    for o in owner.objects.iter().filter(|o| o.special) {
                        match (
                            Self::object_test_vision(sm, alias, o, owner, self, sq),
                            &o.kind,
                        ) {
                            (VisionResult::NoEffect, _) => {}
                            (VisionResult::Unblocked, ObjectKind::IsoDoor(d)) => {
                                ret = if d.open {
                                    TestResults::ClearThroughOpenDoor
                                } else {
                                    TestResults::ClearThroughClosedDoor
                                }
                            }
                            (VisionResult::Unblocked, ObjectKind::IsoWindow(_)) => {
                                ret = TestResults::ClearThroughWindow
                            }
                            (VisionResult::Blocked, ObjectKind::IsoDoor(_)) if !ignore_doors => {
                                return TestResults::Blocked
                            }
                            (
                                VisionResult::Blocked,
                                ObjectKind::IsoCurtain(_) | ObjectKind::IsoWindow(_),
                            ) => return TestResults::Blocked,
                            _ => {}
                        }
                    }
                }
            }
            Some(sq)
                if z > 0
                    && (self.z != -1 || sq.z != 0)
                    && sq.has(F::exterior)
                    && !self.has(F::exterior) =>
            {
                ret = TestResults::Blocked
            }
            _ => {}
        }
        if (self.vision_matrix >> ((x + 1) + (y + 1) * 3 + (z + 1) * 9)) & 1 == 0 {
            ret
        } else {
            TestResults::Blocked
        }
    }
}

/// zombie.iso.LosUtil.lineClear: line of sight between two squares, stepping along the longest axis and testing each
/// pair of consecutive squares with `testVisionAdjacent`. `range_till_windows` limits how far one sees past a window
/// (10000 in the short overload).
#[allow(clippy::too_many_arguments)]
pub fn line_clear<G: GetSquare>(
    sm: &IsoSpriteManager,
    alias: &TilePropertyAliasMap,
    g: &G,
    mut x0: i32,
    mut y0: i32,
    mut z0: i32,
    x1: i32,
    y1: i32,
    mut z1: i32,
    ignore_doors: bool,
    range_till_windows: i32,
    server: bool,
) -> TestResults {
    if z1 == z0 - 1
        && g.get_grid_square(x1, y1, z1)
            .is_some_and(|s| s.has_elevated_floor())
    {
        z1 = z0;
    }
    let mut test = TestResults::Clear;
    let (dy, dx, dz) = (y1 - y0, x1 - x0, z1 - z0);
    let (mut t, mut t2) = (0.5f32, 0.5f32);
    let mut b = g.get_grid_square(x0, y0, z0);
    let mut dist = 0;
    // 0: step x, 1: step y, 2: step z
    let axis = if dx.abs() > dy.abs() && dx.abs() > dz.abs() {
        0
    } else if dy.abs() >= dx.abs() && dy.abs() > dz.abs() {
        1
    } else {
        2
    };
    let (main, o1, o2, c1, c2) = match axis {
        0 => (dx, dy, dz, y0, z0),
        1 => (dy, dx, dz, x0, z0),
        _ => (dz, dx, dy, x0, y0),
    };
    if main == 0 {
        return test;
    }
    let step = if main < 0 { -1 } else { 1 };
    let m = o1 as f32 / main as f32 * step as f32;
    let m2 = o2 as f32 / main as f32 * step as f32;
    t += c1 as f32;
    t2 += c2 as f32;
    loop {
        let done = match axis {
            0 => x0 == x1,
            1 => y0 == y1,
            _ => z0 == z1,
        };
        if done {
            return test;
        }
        t += m;
        t2 += m2;
        let (f1, f2) = (pz_fastfloor(t), pz_fastfloor(t2));
        let a = match axis {
            0 => {
                x0 += step;
                g.get_grid_square(x0, f1, f2)
            }
            1 => {
                y0 += step;
                g.get_grid_square(f1, y0, f2)
            }
            _ => {
                z0 += step;
                g.get_grid_square(f1, f2, z0)
            }
        };
        let mut window_change = false;
        if let (Some(a), Some(b)) = (a, b) {
            let new_test = a.test_vision_adjacent(
                sm,
                alias,
                g,
                b.x - a.x,
                b.y - a.y,
                b.z - a.z,
                true,
                ignore_doors,
                server,
            );
            if new_test == TestResults::ClearThroughWindow {
                window_change = true;
            }
            if new_test != TestResults::Blocked
                && test != TestResults::Clear
                && (new_test != TestResults::ClearThroughWindow
                    || test != TestResults::ClearThroughOpenDoor)
            {
                if new_test == TestResults::ClearThroughClosedDoor
                    && test == TestResults::ClearThroughOpenDoor
                {
                    test = new_test;
                }
            } else {
                test = new_test;
            }
            if test == TestResults::Blocked {
                return TestResults::Blocked;
            }
            if window_change {
                if dist > range_till_windows {
                    return TestResults::Blocked;
                }
                dist = 0;
            }
        }
        b = a;
        dist += 1;
    }
}

fn pz_fastfloor(v: f32) -> i32 {
    crate::zombie::core::math::pz_math::fastfloor(v)
}

impl IsoGridSquare {
    /// zombie.iso.IsoGridSquare.getSlopedSurfaceHeight (float, float): height (0..1) of a ramp at a point of the square.
    pub fn get_sloped_surface_height_at(
        &self,
        alias: &TilePropertyAliasMap,
        dx: f32,
        dy: f32,
    ) -> f32 {
        let Some(dir) = self.get_sloped_surface_direction(alias) else {
            return 0.0;
        };
        let (dx, dy) = (dx.clamp(0.0, 1.0), dy.clamp(0.0, 1.0));
        let min = self.properties.get_sloped_surface_height_min(alias) as f32;
        let max = self.properties.get_sloped_surface_height_max(alias) as f32;
        let lerp = |a: f32| min + (max - min) * a;
        let z = match dir {
            IsoDirections::N => lerp(1.0 - dy),
            IsoDirections::S => lerp(dy),
            IsoDirections::W => lerp(1.0 - dx),
            IsoDirections::E => lerp(dx),
            _ => -1.0,
        };
        if z < 0.0 {
            0.0
        } else {
            z / 100.0
        }
    }

    /// zombie.iso.IsoGridSquare.getApparentZ: the height a character stands at on this square at the point (dx, dy)
    /// inside it: a staircase rises in three squares (bottom, middle, top) towards north or west. `fbo_render_chunk` is
    /// `PerformanceSettings.fboRenderChunk`, which shifts the steps by 0.1.
    pub fn get_apparent_z(
        &self,
        alias: &TilePropertyAliasMap,
        dx: f32,
        dy: f32,
        fbo_render_chunk: bool,
    ) -> f32 {
        let (dx, dy) = (dx.clamp(0.0, 1.0), dy.clamp(0.0, 1.0));
        let fudge: f32 = if fbo_render_chunk { 0.1 } else { 0.0 };
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let z = self.z as f32;
        if self.has_type(T::stairsTN) {
            z + lerp(0.6666 + fudge, 1.0, 1.0 - dy)
        } else if self.has_type(T::stairsTW) {
            z + lerp(0.6666 + fudge, 1.0, 1.0 - dx)
        } else if self.has_type(T::stairsMN) {
            z + lerp(0.3333 + fudge, 0.6666 + fudge, 1.0 - dy)
        } else if self.has_type(T::stairsMW) {
            z + lerp(0.3333 + fudge, 0.6666 + fudge, 1.0 - dx)
        } else if self.has_type(T::stairsBN) {
            z + lerp(0.01, 0.3333 + fudge, 1.0 - dy)
        } else if self.has_type(T::stairsBW) {
            z + lerp(0.01, 0.3333 + fudge, 1.0 - dx)
        } else {
            z + self.get_sloped_surface_height_at(alias, dx, dy)
        }
    }
}
