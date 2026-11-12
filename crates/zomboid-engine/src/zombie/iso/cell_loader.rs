//! zombie.iso.CellLoader: turns the tile names of a map chunk into world objects.
use std::collections::HashMap;

use super::iso_cell;
use super::iso_chunk::IsoChunk;
use super::iso_grid_square::IsoGridSquare;
use super::iso_lot::IsoLot;
use super::iso_meta_grid::IsoMetaGrid;
use super::iso_object::{IsoCurtain, IsoDoor, IsoObject, IsoTree, IsoWindow, ObjectKind};
use super::sprite::iso_sprite::get_sprite_offset;
use super::sprite::iso_sprite_manager::{IsoSpriteManager, SpriteRef};
use super::sprite_details::iso_flag_type::IsoFlagType as F;
use super::sprite_details::iso_object_type::IsoObjectType as T;
use crate::java::{self, JavaRandom};
use crate::zombie::core::math::pz_math;
use crate::zombie::core::properties::iso_property_type as pt;
use crate::zombie::core::random::rand_abstract::RandAbstract;
use crate::zombie::core::tile_property_alias_map::TilePropertyAliasMap;

/// zombie.iso.objects.IsoTree.LOGS_PER_SIZE
const LOGS_PER_SIZE: [i32; 8] = [1, 1, 2, 3, 4, 5, 6, 8];

/// What the loader reads from globals in the original: sprite and property tables, the meta grid, `Rand`, sandbox options.
pub struct LoadContext<'a, R: JavaRandom> {
    /// `IsoSpriteManager.instance`. Mutable: creating a window rewrites flags of the sprites involved.
    pub sprites: &'a mut IsoSpriteManager,
    pub alias: &'a TilePropertyAliasMap,
    /// `CellLoader.glassRemovedWindowSpriteMap`
    pub glass_removed_window_sprite_map: &'a HashMap<SpriteRef, SpriteRef>,
    /// `CellLoader.smashedWindowSpriteMap`
    pub smashed_window_sprite_map: &'a HashMap<SpriteRef, SpriteRef>,
    pub meta_grid: &'a mut IsoMetaGrid,
    /// `Rand` (RandStandard)
    pub rand: &'a mut RandAbstract<R>,
    /// `SandboxOptions.instance.lockedHouses.getValue()` (1..6, default 4)
    pub locked_houses: i32,
    /// Tile names with no sprite definition ("Missing tile definition" in the original's log).
    pub missing_tiles: Vec<String>,
}

fn nfe(s: &str) -> String {
    format!("java.lang.NumberFormatException: For input string: \"{s}\"")
}

/// zombie.iso.CellLoader.AddObject / AddSpecialObject: appends, or inserts where `placeWallAndDoorCheck` says.
/// As in the original, an object that gets inserted is NOT added to the special-object list.
fn add_object(
    sm: &IsoSpriteManager,
    square: &mut IsoGridSquare,
    mut obj: IsoObject,
    special: bool,
) -> usize {
    // GameEntityFactory.CreateIsoEntityFromCellLoading (entity components) is not ported yet.
    let size = square.objects.len() as i32;
    let index = square.place_wall_and_door_check(sm, &obj, size);
    if index != size && index >= 0 && index <= size {
        square.objects.insert(index as usize, obj);
        index as usize
    } else {
        obj.special = special;
        square.objects.push(obj);
        square.objects.len() - 1
    }
}

/// zombie.iso.CellLoader.treatWindowFrameAsOverlay: a window-frame sprite on a square that already has the matching
/// wall becomes an overlay of that wall.
fn treat_window_frame_as_overlay(
    sm: &IsoSpriteManager,
    sprite: SpriteRef,
    square: &mut IsoGridSquare,
) -> bool {
    let (ww, wn) = (
        sm.sprite(sprite).properties.has(F::WindowW),
        sm.sprite(sprite).properties.has(F::WindowN),
    );
    for i in (0..square.objects.len()).rev() {
        let object = &square.objects[i];
        let is_west = object.has_property(sm, F::cutW) && ww;
        let is_north = object.has_property(sm, F::cutN) && wn;
        if object.get_type(sm) == T::wall && (is_west || is_north) {
            square.objects[i].attach(sprite);
            return true;
        }
    }
    false
}

/// zombie.iso.objects.IsoDoor constructor (IsoCell, IsoGridSquare, IsoSprite, boolean).
fn new_door<R: JavaRandom>(
    ctx: &mut LoadContext<R>,
    gid: SpriteRef,
    north: bool,
) -> Result<IsoObject, String> {
    let props = &ctx.sprites.sprite(gid).properties;
    let open = props.has(F::open);
    let mut open_sprite_offset = 2;
    if props.has_str(ctx.alias, pt::DOUBLE_DOOR) {
        open_sprite_offset = 4;
    }
    if props.has_str(ctx.alias, pt::GARAGE_DOOR) {
        open_sprite_offset = 8;
    }
    let closed_sprite = if open {
        get_sprite_offset(ctx.sprites, gid, -open_sprite_offset)?
    } else {
        Some(gid)
    };
    let open_sprite = if open {
        Some(gid)
    } else {
        get_sprite_offset(ctx.sprites, gid, open_sprite_offset)?
    };
    let sprite = if open { open_sprite } else { closed_sprite };
    // `this.getProperties()` is the properties of the chosen sprite; an open door whose sprite is null would throw.
    let sp = sprite.ok_or("java.lang.NullPointerException: door sprite")?;
    let force_locked = ctx
        .sprites
        .sprite(sp)
        .properties
        .has_str(ctx.alias, "forceLocked");
    let rand_lock = match ctx.locked_houses {
        1 => -1,
        2 => 5,
        3 => 10,
        4 => 50,
        5 => 60,
        6 => 70,
        _ => 69,
    };
    let (mut locked, mut locked_by_key) = (false, false);
    if rand_lock > -1 {
        locked = ctx.rand.next(100) < rand_lock;
        if locked && ctx.rand.next(3) == 0 {
            locked_by_key = true;
        }
    }
    if force_locked {
        locked = true;
        locked_by_key = true;
    }
    if open {
        locked = false;
        locked_by_key = false;
    }
    let mut obj = IsoObject::new(
        ObjectKind::IsoDoor(IsoDoor {
            open,
            north,
            locked,
            locked_by_key,
            closed_sprite,
            open_sprite,
        }),
        sprite,
    );
    obj.outline_on_mouseover = true;
    Ok(obj)
}

/// zombie.iso.objects.IsoWindow constructor: besides building the window it rewrites shared sprite data: the closed
/// sprite loses its wall-cut flags and gains the window flag of its edge, and the open/smashed/glass-removed sprites
/// inherit the closed sprite's properties and tile type.
fn new_window<R: JavaRandom>(
    ctx: &mut LoadContext<R>,
    square: &mut IsoGridSquare,
    gid: SpriteRef,
    north: bool,
) -> Result<IsoObject, String> {
    let alias = ctx.alias;
    let int_prop = |sm: &IsoSpriteManager, s: SpriteRef, k: &str| -> Result<Option<i32>, String> {
        match sm.sprite(s).properties.get(alias, k) {
            Some(v) => java::parse_int(v).map(Some).ok_or_else(|| nfe(v)),
            None => Ok(None),
        }
    };
    {
        let p = &mut ctx.sprites.sprite_mut(gid).properties;
        p.unset(F::cutN);
        p.unset(F::cutW);
    }
    let open_offset = int_prop(ctx.sprites, gid, pt::OPEN_TILE_OFFSET)?.unwrap_or(0);
    let perma_locked = ctx
        .sprites
        .sprite(gid)
        .properties
        .has_str(alias, pt::WINDOW_LOCKED);
    let smashed_offset = int_prop(ctx.sprites, gid, pt::SMASHED_TILE_OFFSET)?.unwrap_or(0);
    let closed_sprite = gid;
    {
        let p = &mut ctx.sprites.sprite_mut(closed_sprite).properties;
        if north {
            p.set(F::cutN);
            p.set(F::windowN);
        } else {
            p.set(F::cutW);
            p.set(F::windowW);
        }
    }
    let open_sprite = get_sprite_offset(ctx.sprites, gid, open_offset)?;
    let smashed_sprite = get_sprite_offset(ctx.sprites, gid, smashed_offset)?;
    let glass_removed_sprite = match int_prop(ctx.sprites, closed_sprite, pt::GLASS_REMOVED_OFFSET)?
    {
        Some(off) => get_sprite_offset(ctx.sprites, closed_sprite, off)?,
        None => smashed_sprite,
    };
    for other in [smashed_sprite, open_sprite, glass_removed_sprite]
        .into_iter()
        .flatten()
    {
        if other != closed_sprite {
            let closed = ctx.sprites.sprite(closed_sprite).clone();
            let o = ctx.sprites.sprite_mut(other);
            o.properties.add_properties(&closed.properties);
            o.tile_type = closed.tile_type;
        }
    }
    if let Some(wall) = square.get_wall(ctx.sprites, north) {
        square.objects[wall].reroute_collide = true;
    }
    // zombie.iso.objects.IsoWindow.LockedHouseFrequency: values 0..5, anything else is VeryOften. (Doors read the same
    // sandbox option with a table shifted by one.)
    let rand_lock = match ctx.locked_houses {
        0 => 0,
        1 => 5,
        2 => 10,
        3 => 50,
        4 => 60,
        _ => 70,
    };
    let mut locked = false;
    if rand_lock > 0 {
        locked = ctx.rand.next(100) < rand_lock;
    }
    Ok(IsoObject::new(
        ObjectKind::IsoWindow(IsoWindow {
            north,
            locked,
            perma_locked,
            smashed: false,
            glass_removed: false,
            closed_sprite: Some(closed_sprite),
            open_sprite,
            smashed_sprite,
            glass_removed_sprite,
        }),
        Some(closed_sprite),
    ))
}

/// zombie.iso.CellLoader.DoTileObjectCreation: creates the object for one tile on `sq`, or attaches the tile to an
/// object already there (overlays), or drops it. `name` is the tile name. `Err` is an exception of the original, which
/// aborts the whole chunk placement.
pub fn do_tile_object_creation<R: JavaRandom>(
    ctx: &mut LoadContext<R>,
    mut spr: SpriteRef,
    mut ty: T,
    sq: &mut IsoGridSquare,
    name: &str,
) -> Result<(), String> {
    let alias = ctx.alias;
    let mut b_glass_removed_window = false;
    let mut b_smashed_window = false;
    if let Some(s) = ctx.glass_removed_window_sprite_map.get(&spr) {
        spr = *s;
        ty = ctx.sprites.sprite(spr).tile_type;
        b_glass_removed_window = true;
    } else if let Some(s) = ctx.smashed_window_sprite_map.get(&spr) {
        spr = *s;
        ty = ctx.sprites.sprite(spr).tile_type;
        b_smashed_window = true;
    }

    let has = |sm: &IsoSpriteManager, f: F| sm.sprite(spr).properties.has(f);
    let container = ctx
        .sprites
        .sprite(spr)
        .properties
        .get(alias, pt::CONTAINER)
        .map(str::to_string);
    let container_is = |v: &str| container.as_deref() == Some(v);
    let has_container = has(ctx.sprites, F::container);
    let eq_ic = |v: &str| {
        container
            .as_deref()
            .is_some_and(|c| java::equals_ignore_case(c, v))
    };
    let is_stove = eq_ic("stove") || eq_ic("toaster") || eq_ic("coffeemaker");
    let spr_name = ctx.sprites.sprite(spr).name.clone();

    if ctx.sprites.sprite(spr).solidfloor
        && has(ctx.sprites, F::diamondFloor)
        && !has(ctx.sprites, F::transparentFloor)
    {
        if let Some(floor) = sq.get_floor(ctx.sprites) {
            if sq.objects[floor].has_property(ctx.sprites, F::diamondFloor) {
                // Entity components on the old floor (not ported) would be removed first in the original.
                sq.objects[floor].attached_anim_sprite = None;
                sq.objects[floor].sprite = Some(spr);
                return Ok(());
            }
        }
    }

    let obj: Option<usize>;
    if ty == T::doorW || ty == T::doorN {
        let mut door = new_door(ctx, spr, ty == T::doorN)?;
        if let ObjectKind::IsoDoor(d) = &mut door.kind {
            if ctx
                .sprites
                .sprite(spr)
                .properties
                .has_str(alias, pt::DOUBLE_DOOR)
            {
                d.locked = false;
                d.locked_by_key = false;
            }
            if ctx
                .sprites
                .sprite(spr)
                .properties
                .has_str(alias, pt::GARAGE_DOOR)
            {
                d.locked = !d.open;
                d.locked_by_key = false;
            }
        }
        obj = Some(add_object(ctx.sprites, sq, door, true));
    } else if ty == T::lightswitch {
        // The light source and room wiring of IsoLightSwitch are not ported yet.
        obj = Some(add_object(
            ctx.sprites,
            sq,
            IsoObject::new(ObjectKind::IsoLightSwitch, Some(spr)),
            false,
        ));
    } else if ty != T::curtainN && ty != T::curtainS && ty != T::curtainE && ty != T::curtainW {
        if has(ctx.sprites, F::windowW) || has(ctx.sprites, F::windowN) {
            let north = has(ctx.sprites, F::windowN);
            let mut w = new_window(ctx, sq, spr, north)?;
            if let ObjectKind::IsoWindow(win) = &mut w.kind {
                if b_glass_removed_window {
                    win.smashed = true;
                    win.glass_removed = true;
                } else if b_smashed_window {
                    win.smashed = true;
                }
            }
            obj = Some(add_object(ctx.sprites, sq, w, true));
        } else if !has(ctx.sprites, F::WindowW) && !has(ctx.sprites, F::WindowN) {
            // `get(CONTAINER).equals(..)` of the original throws when the flag is set without a value.
            let c = |v: &str| -> Result<bool, String> {
                if container.is_none() {
                    return Err("java.lang.NullPointerException: container".into());
                }
                Ok(container_is(v))
            };
            let simple = |ctx: &mut LoadContext<R>, sq: &mut IsoGridSquare, kind: ObjectKind| {
                Some(add_object(
                    ctx.sprites,
                    sq,
                    IsoObject::new(kind, Some(spr)),
                    false,
                ))
            };
            // The container-driven classes, tested in the original's order.
            let iso_type_cwd = ctx.sprites.sprite(spr).properties.get(alias, pt::ISO_TYPE)
                == Some("IsoCombinationWasherDryer");
            let container_kind = (|| -> Result<Option<ObjectKind>, String> {
                if has_container && (c("barbecue")? || c("barbecuepropane")?) {
                    return Ok(Some(ObjectKind::IsoBarbecue {
                        has_propane_tank: false,
                        fuel_amount_minutes: 0,
                    }));
                }
                if has_container && c("fireplace")? {
                    return Ok(Some(ObjectKind::IsoFireplace));
                }
                let n = spr_name
                    .as_deref()
                    .ok_or("java.lang.NullPointerException: spr.getName()")?;
                if n != "camping_01_04"
                    && n != "camping_01_05"
                    && n != "camping_01_06"
                    && has_container
                    && c("campfire")?
                {
                    return Ok(Some(ObjectKind::IsoFireplace));
                }
                if container_is("brazier") {
                    return Ok(Some(ObjectKind::IsoFireplace));
                }
                if iso_type_cwd {
                    return Ok(Some(ObjectKind::IsoCombinationWasherDryer));
                }
                if has_container && c("clothingdryer")? {
                    return Ok(Some(ObjectKind::IsoClothingDryer));
                }
                if has_container && c("clothingwasher")? {
                    return Ok(Some(ObjectKind::IsoClothingWasher));
                }
                if has_container && c("woodstove")? {
                    return Ok(Some(ObjectKind::IsoFireplace));
                }
                if has_container && (is_stove || c("microwave")?) {
                    return Ok(Some(ObjectKind::IsoStove));
                }
                Ok(None)
            })()?;
            if let Some(mut kind) = container_kind {
                if let ObjectKind::IsoBarbecue {
                    has_propane_tank,
                    fuel_amount_minutes,
                } = &mut kind
                {
                    // zombie.iso.objects.IsoBarbecue constructor: a propane barbecue starts with a random amount of fuel.
                    // isSpriteWithPropaneTank: this sprite and the one 8 tiles further both carry "propaneTank".
                    let with_tank = ctx
                        .sprites
                        .sprite(spr)
                        .properties
                        .has_str(alias, pt::PROPANE_TANK)
                        && get_sprite_offset(ctx.sprites, spr, 8)?.is_some_and(|s2| {
                            ctx.sprites
                                .sprite(s2)
                                .properties
                                .has_str(alias, pt::PROPANE_TANK)
                        });
                    if with_tank {
                        *has_propane_tank = true;
                        *fuel_amount_minutes = ctx.rand.next(1201);
                    }
                }
                obj = simple(ctx, sq, kind);
            } else if ty == T::jukebox {
                let i = simple(ctx, sq, ObjectKind::IsoJukebox);
                if let Some(i) = i {
                    sq.objects[i].outline_on_mouseover = true;
                }
                obj = i;
            } else if ty == T::radio {
                obj = simple(ctx, sq, ObjectKind::IsoRadio);
            } else if ctx.sprites.sprite(spr).properties.has_str(alias, "signal") {
                obj = match ctx.sprites.sprite(spr).properties.get(alias, "signal") {
                    Some("radio") => simple(ctx, sq, ObjectKind::IsoRadio),
                    Some("tv") => simple(ctx, sq, ObjectKind::IsoTelevision),
                    // The original passes null to AddObject here.
                    _ => return Err("java.lang.NullPointerException: signal object".into()),
                };
            } else if ctx
                .sprites
                .sprite(spr)
                .properties
                .has_str(alias, pt::COUNTERTOP)
            {
                match sq.get_countertop_attach_object(ctx.sprites, alias) {
                    Some(i) => {
                        sq.objects[i].attach(spr);
                        obj = None;
                    }
                    None => obj = simple(ctx, sq, ObjectKind::IsoObject),
                }
            } else if has(ctx.sprites, F::WallOverlay) {
                let mut obj2: Option<usize> = None;
                if has(ctx.sprites, F::attachedSE) {
                    obj2 = sq.get_wall_se(ctx.sprites);
                } else if has(ctx.sprites, F::attachedW) || has(ctx.sprites, F::attachedN) {
                    let north = !has(ctx.sprites, F::attachedW);
                    obj2 = sq.get_wall(ctx.sprites, north);
                    if obj2.is_none() {
                        obj2 = sq.get_window(north).filter(|i| {
                            sq.objects[*i].has_property(
                                ctx.sprites,
                                if north { F::WindowN } else { F::WindowW },
                            )
                        });
                    }
                    if obj2.is_none() {
                        obj2 = sq.get_garage_door(ctx.sprites, alias, north)?;
                    }
                } else {
                    for n in (0..sq.objects.len()).rev() {
                        // The original dereferences obj3.sprite without a null check.
                        let s3 = sq.objects[n]
                            .sprite
                            .ok_or("java.lang.NullPointerException: obj3.sprite")?;
                        let p3 = &ctx.sprites.sprite(s3).properties;
                        if p3.has(F::cutW) || p3.has(F::cutN) {
                            obj2 = Some(n);
                            break;
                        }
                    }
                }
                match obj2 {
                    Some(i) => sq.objects[i].attach(spr),
                    None => {
                        add_object(
                            ctx.sprites,
                            sq,
                            IsoObject::new(ObjectKind::IsoObject, Some(spr)),
                            false,
                        );
                    }
                }
                // The original returns here: an overlay object gets no tile name, container or tint.
                return Ok(());
            } else if has(ctx.sprites, F::FloorOverlay) {
                if let Some(i) = sq.get_floor(ctx.sprites) {
                    sq.objects[i].attach(spr);
                }
                obj = None;
            } else if ctx
                .sprites
                .sprite(spr)
                .properties
                .get(alias, pt::CUSTOM_NAME)
                == Some("Mannequin")
            {
                obj = simple(ctx, sq, ObjectKind::IsoMannequin);
            } else if ty == T::tree {
                if spr_name
                    .as_deref()
                    .is_some_and(|n| n.starts_with("vegetation_trees"))
                {
                    let ok = sq
                        .get_floor(ctx.sprites)
                        .and_then(|f| sq.objects[f].sprite)
                        .and_then(|s| ctx.sprites.sprite(s).name.as_deref())
                        .is_some_and(|n| n.starts_with("blends_natural"));
                    if !ok {
                        return Ok(());
                    }
                }
                // zombie.iso.objects.IsoTree.initTree
                let size = match ctx.sprites.sprite(spr).properties.get(alias, "tree") {
                    Some(v) => java::parse_int(v).ok_or_else(|| nfe(v))?.clamp(1, 8),
                    None => 4,
                };
                let log_yield = LOGS_PER_SIZE[(size - 1) as usize];
                let damage = ((log_yield - 1) * 80).max(40);
                if let Some(i) = sq
                    .objects
                    .iter()
                    .position(|o| matches!(o.kind, ObjectKind::IsoTree(_)))
                {
                    sq.objects.remove(i);
                }
                obj = simple(
                    ctx,
                    sq,
                    ObjectKind::IsoTree(IsoTree {
                        size,
                        log_yield,
                        damage,
                    }),
                );
            } else {
                // Client only: a sprite without a texture is replaced by the "missing tile" image or dropped. Not ported;
                // this follows the dedicated server, which keeps every object.
                obj = simple(ctx, sq, ObjectKind::IsoObject);
            }
        } else if !treat_window_frame_as_overlay(ctx.sprites, spr, sq) {
            let north = has(ctx.sprites, F::WindowN);
            obj = Some(add_object(
                ctx.sprites,
                sq,
                IsoObject::new(ObjectKind::IsoWindowFrame { north }, Some(spr)),
                false,
            ));
        } else {
            obj = None;
        }
    } else {
        let tail = &name[name.rfind('_').map_or(0, |i| i + 1)..];
        let closed = java::parse_int(tail).ok_or_else(|| nfe(tail))? % 8 <= 3;
        // zombie.iso.objects.IsoCurtain constructor: the curtain starts open, showing the sprite 4 tiles after a closed one.
        let (open_sprite, closed_sprite) = if closed {
            (get_sprite_offset(ctx.sprites, spr, 4)?, Some(spr))
        } else {
            (Some(spr), get_sprite_offset(ctx.sprites, spr, -4)?)
        };
        let north = ty == T::curtainN || ty == T::curtainS;
        let mut o = IsoObject::new(
            ObjectKind::IsoCurtain(IsoCurtain {
                north,
                open: true,
                closed_sprite,
                open_sprite,
            }),
            open_sprite,
        );
        o.outline_on_mouseover = true;
        obj = Some(add_object(ctx.sprites, sq, o, true));
    }

    if let Some(i) = obj {
        sq.objects[i].tile = Some(name.to_string());
        // createContainersFromSpriteProperties (ItemContainer) is not ported yet.
        // A curtain or door whose shown sprite is missing would throw in the original here.
        let s = sq.objects[i]
            .sprite
            .ok_or("java.lang.NullPointerException: obj.sprite")?;
        if ctx.sprites.sprite(s).properties.has(F::vegitation) {
            sq.objects[i].tintr = 0.7 + ctx.rand.next(30) as f32 / 100.0;
            sq.objects[i].tintg = 0.7 + ctx.rand.next(30) as f32 / 100.0;
            sq.objects[i].tintb = 0.7 + ctx.rand.next(30) as f32 / 100.0;
        }
    }
    Ok(())
}

/// zombie.iso.CellLoader.LoadCellBinaryChunk (IsoCell, int, int, IsoChunk): fills `chunk` (world chunk coordinates
/// `wx`, `wy`) from the map folders, highest priority first; lower-priority folders fill the columns left empty.
/// `false` when no map folder has the cell. Errors of single placements are returned as text.
pub fn load_cell_binary_chunk<R: JavaRandom>(
    ctx: &mut LoadContext<R>,
    wx: i32,
    wy: i32,
    chunk: &mut IsoChunk,
    errors: &mut Vec<String>,
) -> bool {
    let cell_x = pz_math::fastfloor(wx as f32 / 32.0);
    let cell_y = pz_math::fastfloor(wy as f32 / 32.0);
    let filenamepack = format!("world_{cell_x}_{cell_y}.lotpack");
    let header_name = format!("{cell_x}_{cell_y}.lotheader");
    let Some(first) = ctx
        .meta_grid
        .map_files
        .iter()
        .position(|m| m.info_file_names.contains_key(&filenamepack))
    else {
        return false;
    };
    let mut b_done_squares = [false; 64];
    let mut lot = IsoLot::default();
    let mut placed_first = false;
    for i in first..ctx.meta_grid.map_files.len() {
        if placed_first && !ctx.meta_grid.map_files[i].has_cell(cell_x, cell_y) {
            continue;
        }
        let path = ctx.meta_grid.map_files[i]
            .info_file_names
            .get(&filenamepack)
            .cloned();
        // The header leaves the grid while the chunk is placed, because placing creates runtime rooms in the grid.
        let info = ctx.meta_grid.map_files[i].info_headers.remove(&header_name);
        let (Some(path), Some(info)) = (path, info) else {
            if !placed_first {
                // `lot.info == null`: the original returns true without placing anything.
                return true;
            }
            continue;
        };
        let pack = std::fs::read(&path);
        if let Ok(pack) = &pack {
            if let Some(e) = lot.load(&info, pack, cell_x, cell_y, wx, wy) {
                errors.push(e);
            }
            let (non_empty_square_count, err) = iso_cell::place_lot(
                ctx,
                &lot,
                &info,
                0,
                0,
                lot.min_level,
                chunk,
                wx,
                wy,
                &mut b_done_squares,
            );
            errors.extend(err);
            ctx.meta_grid.map_files[i]
                .info_headers
                .insert(header_name.clone(), info);
            placed_first = true;
            if non_empty_square_count == 64 {
                break;
            }
        } else {
            ctx.meta_grid.map_files[i]
                .info_headers
                .insert(header_name.clone(), info);
            if !placed_first {
                return false;
            }
        }
    }
    true
}
