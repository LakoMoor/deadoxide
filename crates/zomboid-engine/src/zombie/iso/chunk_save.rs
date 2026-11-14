//! Loading a chunk from a save (`map/<wx>/<wy>.bin`): zombie.iso.IsoChunk.LoadFromDiskOrBufferInternal,
//! zombie.iso.IsoGridSquare.load, zombie.iso.IsoObject.load and the `load` of each zombie.iso.objects class a vanilla
//! save holds. Object state that the scene already models (door open, window smashed, ...) goes into `ObjectKind`;
//! the rest of what the save carries is kept in `ObjectSave` / `SquareSave` / `ChunkSave`.
use super::iso_chunk::IsoChunk;
use super::iso_grid_square::IsoGridSquare;
use super::iso_meta_grid::IsoMetaGrid;
use super::iso_object::{IsoCurtain, IsoDoor, IsoObject, IsoTree, IsoWindow, ObjectKind};
use super::objects::iso_dead_body::IsoDeadBody;
use super::sprite::iso_sprite_manager::{IsoSpriteManager, SpriteRef};
use super::sprite_details::iso_flag_type::IsoFlagType as F;
use crate::java::{self, JavaRandom};
use crate::java_nio::{
    read_string, unpack_byte_to_float_unit, BitHeaderRead, ByteBlock, ByteBuffer, HeaderSize,
};
use crate::se::krka::kahlua::j2se::kahlua_table_impl::KahluaTable;
use crate::zombie::core::properties::iso_property_type as pt;
use crate::zombie::core::random::rand_abstract::RandAbstract;
use crate::zombie::core::skinnedmodel::visual::human_visual::HumanVisual;
use crate::zombie::core::tile_property_alias_map::TilePropertyAliasMap;
use crate::zombie::erosion::erosion_data;
use crate::zombie::inventory::inventory_item::{InventoryItem, ItemFactory};
use crate::zombie::inventory::item_container::ItemContainer;
use crate::zombie::radio::devices::device_data::DeviceData;
use crate::zombie::scripting::item::parse_float;
use crate::zombie::world::dictionary_data::DictionaryData;

/// The highest world version this build reads (`IsoChunk.LoadFromDiskOrBufferInternal` refuses newer chunks).
pub const WORLD_VERSION: i32 = 249;

/// What chunk loading reads from the running game.
pub struct SaveLoadContext<'a, R: JavaRandom> {
    /// `IsoSpriteManager.instance`. Mutable: loading a window rewrites flags of its sprites, an overlay sprite name
    /// creates its sprite.
    pub sprites: &'a mut IsoSpriteManager,
    pub alias: &'a TilePropertyAliasMap,
    /// `WorldDictionary`: object-name and sprite-name ids.
    pub dictionary: &'a DictionaryData,
    /// `InventoryItemFactory.CreateItem(short)` and `ScriptManager.FindItem(..).getObsolete()`
    pub items: &'a dyn ItemFactory,
    /// `IsoWorld.instance.getMetaGrid()`: room ids of the squares.
    pub meta_grid: &'a mut IsoMetaGrid,
    /// `Rand`, drawn when a square's room creates its building.
    pub rand: &'a mut RandAbstract<R>,
    /// `IsoLot.InfoHeaders.containsKey(header of the chunk's 256x256 cell)`: only then do squares get room ids.
    pub has_lot_header: bool,
}

/// zombie.iso.IsoFloorBloodSplat as saved.
#[derive(Clone, Debug, PartialEq)]
pub struct FloorBloodSplat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r#type: i32,
    /// As saved. The original clamps it to the current world age and drops splats older than the sandbox's
    /// `BloodSplatLifespanDays`; neither is applied here.
    pub world_age: f32,
    pub index: i32,
}

/// The chunk-level part of a chunk save that the scene does not use.
#[derive(Clone, Debug, PartialEq)]
pub struct ChunkSave {
    pub world_version: i32,
    /// The saved length or CRC differs from the bytes read (the original only logs this).
    pub length_mismatch: bool,
    pub crc_mismatch: bool,
    pub blending_done_full: bool,
    pub blending_modified: [bool; 4],
    pub blending_done_partial: bool,
    pub blending_depth: [i8; 4],
    pub attachments_done_full: bool,
    pub attachments_state: [bool; 5],
    /// zombie.iso.IsoChunk.attachmentsPartial: SquareCoord (x, y, z)
    pub attachments_partial: Option<Vec<(i32, i32, i32)>>,
    pub floor_blood_splats: Vec<FloorBloodSplat>,
    pub erosion: erosion_data::Chunk,
    pub generators_touching_this_chunk: Option<Vec<(i32, i32, i32)>>,
    pub loot_respawn_hour: i32,
    pub spawned_rooms: Vec<i64>,
}

impl Default for ChunkSave {
    /// The field initialisers of zombie.iso.IsoChunk (`blendingDepth` is `BlendDirection.*.defaultDepth`).
    fn default() -> Self {
        ChunkSave {
            world_version: 0,
            length_mismatch: false,
            crc_mismatch: false,
            blending_done_full: false,
            blending_modified: [false; 4],
            blending_done_partial: false,
            blending_depth: [7, 0, 7, 0],
            attachments_done_full: true,
            attachments_state: [true; 5],
            attachments_partial: None,
            floor_blood_splats: Vec::new(),
            erosion: erosion_data::Chunk {
                e_tick_stamp: -1,
                epoch: -1,
                ..erosion_data::Chunk::default()
            },
            generators_touching_this_chunk: None,
            loot_respawn_hour: -1,
            spawned_rooms: Vec::new(),
        }
    }
}

/// The square-level part of a chunk save that the scene does not use.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SquareSave {
    pub erosion: erosion_data::Square,
    pub overlay_done: bool,
    pub burnt_out: bool,
    pub have_sheet_rope: bool,
    pub mod_data: Option<KahluaTable>,
    /// zombie.iso.IsoGridSquare.worldObjects: indices into `objects`.
    pub world_objects: Vec<usize>,
    /// zombie.iso.IsoGridSquare.staticMovingObjects (only corpses in vanilla saves).
    pub static_moving_objects: Vec<IsoDeadBody>,
    /// The `vis` byte (`setIsSeen` per player; ignored by clients and servers).
    pub vis: i8,
}

/// An attached sprite (zombie.iso.sprite.IsoSpriteInstance) as saved.
#[derive(Clone, Debug, PartialEq)]
pub struct AttachedSave {
    pub flip: bool,
    pub copy_target_alpha: bool,
    pub multiply_object_alpha: bool,
    /// offX, offY, offZ, tintr, tintg, tintb (`None`: 0, 0, 0, 1, 1, 1)
    pub data: Option<[f32; 6]>,
    pub alpha: Option<f32>,
}

/// zombie.iso.IsoWallBloodSplat: a blood overlay on a wall.
#[derive(Clone, Debug, PartialEq)]
pub struct WallBloodSplat {
    /// As saved; `None` for an "overlay_blood_*" attached sprite, which the original dates to the current world age.
    pub world_age: Option<f32>,
    pub sprite: Option<SpriteRef>,
}

/// zombie.iso.objects.IsoLightSwitch, the saved fields.
#[derive(Clone, Debug, PartialEq)]
pub struct LightSwitchSave {
    pub light_room: bool,
    pub room_id: i64,
    pub activated: bool,
    /// `activated` came from `room.def.lightsActive` (the square has a room and the switch lights it).
    pub activated_from_room: bool,
    pub can_be_modified: bool,
    pub use_battery: bool,
    pub has_battery: bool,
    pub bulb_item: Option<String>,
    pub power: f32,
    pub delta: f32,
    pub primary: [f32; 3],
    pub last_minute_stamp: i64,
    pub bulb_burn_minutes: i32,
    pub street_light: bool,
}

impl Default for LightSwitchSave {
    /// The field initialisers of zombie.iso.objects.IsoLightSwitch.
    fn default() -> Self {
        LightSwitchSave {
            light_room: false,
            room_id: -1,
            activated: false,
            activated_from_room: false,
            can_be_modified: false,
            use_battery: false,
            has_battery: false,
            bulb_item: Some("Base.LightBulb".into()),
            power: 0.0,
            delta: 2.5e-4,
            primary: [1.0; 3],
            last_minute_stamp: -1,
            bulb_burn_minutes: -1,
            street_light: false,
        }
    }
}

/// zombie.iso.objects.IsoMannequin, the saved fields (its container is the object's first container).
#[derive(Clone, Debug, PartialEq)]
pub struct MannequinSave {
    pub dir: i32,
    pub init: bool,
    pub female: bool,
    pub zombie: bool,
    pub skeleton: bool,
    pub mannequin_script_name: String,
    pub pose: String,
    pub human_visual: HumanVisual,
    /// (body location, index into the saved items)
    pub worn_items: Vec<(String, i16)>,
}

/// zombie.iso.objects.IsoWorldInventoryObject, the saved fields.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldInventorySave {
    pub xoff: f32,
    pub yoff: f32,
    pub zoff: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub item: Option<InventoryItem>,
    pub drop_time: f64,
    pub ignore_remove_sandbox: bool,
    pub extended_placement: bool,
}

/// The saved fields of the object's class beyond what `ObjectKind` holds.
#[derive(Clone, Debug, PartialEq)]
pub enum ClassSave {
    None,
    Door {
        health: i32,
        max_health: i32,
        key_id: i32,
        has_curtain: bool,
        curtain_open: bool,
        curtain_inside: bool,
    },
    Window {
        health: i32,
        max_health: i32,
    },
    Curtain {
        health: i32,
        barricade_strength: i32,
    },
    LightSwitch(LightSwitchSave),
    Barbecue {
        lit: bool,
        last_update_time: f32,
        minutes_since_extinguished: i32,
        normal: Option<SpriteRef>,
        no_tank: Option<SpriteRef>,
    },
    Fireplace {
        fuel_amount_minutes: i32,
        lit: bool,
        last_update_time: f32,
        minutes_since_extinguished: i32,
    },
    Stove {
        activated: bool,
        seconds_timer: i32,
        max_temperature: f32,
        first_turn_on: bool,
        broken: bool,
    },
    /// ClothingWasherLogic
    Washer {
        activated: bool,
        last_update: f32,
    },
    /// ClothingDryerLogic
    Dryer {
        activated: bool,
    },
    /// IsoCombinationWasherDryer: `dryer` selects the active logic.
    WasherDryer {
        dryer: bool,
        washer_activated: bool,
        washer_last_update: f32,
        dryer_activated: bool,
    },
    WaveSignal(Option<DeviceData>),
    Mannequin(Box<MannequinSave>),
    WorldInventory(Box<WorldInventorySave>),
}

/// The part of zombie.iso.IsoObject that only saves set.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectSave {
    pub sprite_id: i32,
    pub attached: Vec<AttachedSave>,
    pub wall_blood_splats: Vec<WallBloodSplat>,
    pub name: Option<String>,
    pub sprite_name: Option<String>,
    pub custom_color: Option<[f32; 3]>,
    pub do_not_sync: bool,
    /// The object's container, then its secondary containers.
    pub containers: Vec<ItemContainer>,
    pub mod_data: Option<KahluaTable>,
    pub special_tooltip: bool,
    pub key_id: i32,
    pub uses_external_water_source: bool,
    pub sheet_rope_health: Option<f32>,
    pub overlay_sprite: Option<SpriteRef>,
    pub overlay_sprite_color: Option<[f32; 4]>,
    pub moved_thumpable: bool,
    /// GameEntity components: (component id, raw bytes after the id).
    pub components: Vec<(i16, Vec<u8>)>,
    pub sprite_model_name: Option<String>,
    pub class: ClassSave,
}

impl Default for ObjectSave {
    fn default() -> Self {
        ObjectSave {
            sprite_id: -1,
            attached: Vec::new(),
            wall_blood_splats: Vec::new(),
            name: None,
            sprite_name: None,
            custom_color: None,
            do_not_sync: false,
            containers: Vec::new(),
            mod_data: None,
            special_tooltip: false,
            key_id: -1,
            uses_external_water_source: false,
            sheet_rope_health: None,
            overlay_sprite: None,
            overlay_sprite_color: None,
            moved_thumpable: false,
            components: Vec::new(),
            sprite_model_name: None,
            class: ClassSave::None,
        }
    }
}

/// zombie.iso.IsoChunk.readFlags
fn read_flags<const N: usize>(b: &mut ByteBuffer) -> Result<[bool; N], String> {
    let val = i32::from(b.get()?);
    let mut ret = [false; N];
    let mut pow = 1;
    for r in &mut ret {
        *r = val & pow == pow;
        pow *= 2;
    }
    Ok(ret)
}

/// zombie.entity.GameEntity.loadEntity: a count, then one ByteBlock per component. The component loads are not
/// ported; each block is kept as (component id, bytes) and skipped as `safelyForceSkipOnEnd` allows.
pub fn load_entity(b: &mut ByteBuffer) -> Result<Vec<(i16, Vec<u8>)>, String> {
    let count = b.get()?;
    let mut out = Vec::new();
    for _ in 0..count.max(0) {
        let block = ByteBlock::start(b)?;
        let start = b.position();
        let id = b.get_short()?;
        block.skip_and_end(b)?;
        let end = b.position();
        out.push((id, b.array().get(start + 2..end).unwrap_or(&[]).to_vec()));
    }
    Ok(out)
}

/// zombie.iso.objects.IsoWindow.load (sprite part): the closed sprite gets the window flags of its side and lends its
/// properties and type to the smashed, open and glass-removed sprites.
fn window_sprites(
    sm: &mut IsoSpriteManager,
    north: bool,
    closed: Option<SpriteRef>,
    others: [Option<SpriteRef>; 3],
) {
    let Some(closed) = closed else { return };
    {
        let p = &mut sm.sprite_mut(closed).properties;
        if north {
            p.set(F::cutN);
            p.set(F::windowN);
        } else {
            p.set(F::cutW);
            p.set(F::windowW);
        }
    }
    for other in others.into_iter().flatten() {
        if other != closed {
            let props = sm.sprite(closed).properties.clone();
            let tile_type = sm.sprite(closed).tile_type;
            let o = sm.sprite_mut(other);
            o.properties.add_properties(&props);
            o.tile_type = tile_type;
        }
    }
}

/// `input.get() != 0 ? IsoSprite.getSprite(manager, input.getInt())` (`None`: the flag byte was 0).
fn optional_sprite(
    b: &mut ByteBuffer,
    sm: &IsoSpriteManager,
) -> Result<Option<Option<SpriteRef>>, String> {
    Ok(if b.get()? != 0 {
        Some(sm.get_sprite_by_id(b.get_int()?))
    } else {
        None
    })
}

fn nfe(v: &str) -> String {
    format!("java.lang.NumberFormatException: For input string: \"{v}\"")
}

impl IsoObject {
    /// zombie.iso.IsoObject.factoryFromFileInput (ByteBuffer): the serialise flag and the class id. `Ok(None)` for an
    /// object saved as "not serialised". Class ids without a factory make a plain IsoObject, as the original does
    /// outside debug mode; classes whose load is not ported are `Err`.
    pub fn factory_from_file_input(b: &mut ByteBuffer) -> Result<Option<ObjectKind>, String> {
        if b.get()? == 0 {
            return Ok(None);
        }
        let class_id = b.get()?;
        Ok(Some(match class_id {
            6 => ObjectKind::IsoWorldInventoryObject,
            8 => ObjectKind::IsoCurtain(IsoCurtain::default()),
            9 => ObjectKind::IsoRadio,
            10 => ObjectKind::IsoTelevision,
            12 => ObjectKind::IsoBarbecue {
                has_propane_tank: false,
                fuel_amount_minutes: 0,
            },
            13 => ObjectKind::IsoClothingDryer,
            14 => ObjectKind::IsoClothingWasher,
            15 => ObjectKind::IsoFireplace,
            16 => ObjectKind::IsoStove,
            17 => ObjectKind::IsoDoor(IsoDoor::default()),
            20 => ObjectKind::IsoBrokenGlass,
            24 => ObjectKind::IsoMannequin,
            26 => ObjectKind::IsoWindow(IsoWindow::default()),
            28 => ObjectKind::IsoTree(IsoTree::default()),
            29 => ObjectKind::IsoLightSwitch,
            34 => ObjectKind::IsoCombinationWasherDryer,
            41 => ObjectKind::IsoWindowFrame { north: false },
            0 | 25 | 42.. | i8::MIN..=-1 => ObjectKind::IsoObject,
            _ => {
                return Err(format!(
                    "loading saved objects of class id {class_id} is not ported"
                ))
            }
        }))
    }

    /// zombie.iso.IsoObject.load, then the subclass part (IsoWorldInventoryObject has its own format).
    /// `square_room`: `square.getRoom()` of the square being loaded.
    pub fn load<R: JavaRandom>(
        &mut self,
        b: &mut ByteBuffer,
        world_version: i32,
        ctx: &mut SaveLoadContext<R>,
        square_room: Option<usize>,
    ) -> Result<(), String> {
        if matches!(self.kind, ObjectKind::IsoWorldInventoryObject) {
            return self.load_world_inventory_object(b, ctx);
        }
        let mut s = ObjectSave::default();
        // IsoChunk.Fix2x leaves ids alone while a chunk loads (`fixed2x` is set first).
        let id = b.get_int()?;
        s.sprite_id = id;
        self.sprite = if id == -1 {
            Some(ctx.sprites.get_sprite(""))
        } else {
            ctx.sprites.get_sprite_by_id(id)
        };
        let header = BitHeaderRead::alloc_read(HeaderSize::Byte, b)?;
        if !header.equals(0) {
            if header.has_flags(1)? {
                let child = if header.has_flags(2)? {
                    1
                } else {
                    i32::from(b.get()?) & 255
                };
                for _ in 0..child {
                    if self.attached_anim_sprite.is_none() {
                        self.attached_anim_sprite = Some(Vec::new());
                    }
                    let spr = ctx.sprites.get_sprite_by_id(b.get_int()?);
                    let flags = i32::from(b.get()?);
                    let data = if flags & 2 != 0 {
                        let (x, y, z) = (b.get_float()?, b.get_float()?, b.get_float()?);
                        let r = unpack_byte_to_float_unit(b.get()?);
                        let g = unpack_byte_to_float_unit(b.get()?);
                        let bl = unpack_byte_to_float_unit(b.get()?);
                        Some([x, y, z, r, g, bl])
                    } else {
                        None
                    };
                    let alpha = if flags & 16 != 0 {
                        Some(b.get_float()?)
                    } else {
                        None
                    };
                    if let Some(spr) = spr {
                        if ctx
                            .sprites
                            .sprite(spr)
                            .name
                            .as_deref()
                            .is_some_and(|n| n.starts_with("overlay_blood_"))
                        {
                            s.wall_blood_splats.push(WallBloodSplat {
                                world_age: None,
                                sprite: Some(spr),
                            });
                        } else {
                            if let Some(a) = self.attached_anim_sprite.as_mut() {
                                a.push(spr);
                            }
                            s.attached.push(AttachedSave {
                                flip: flags & 4 != 0,
                                // reset() sets copyTargetAlpha; flag 8 only sets it again.
                                copy_target_alpha: true,
                                multiply_object_alpha: flags & 16 != 0,
                                data,
                                alpha,
                            });
                        }
                    }
                }
            }
            if header.has_flags(4)? {
                let flags = i32::from(b.get()?);
                if flags & 2 != 0 {
                    s.name = Some("Grass".into());
                } else if flags & 4 != 0 {
                    s.name = ctx
                        .dictionary
                        .get_object_name_from_id(b.get()?)
                        .map(str::to_string);
                } else if flags & 8 != 0 {
                    s.name = Some(read_string(b)?);
                }
                if flags & 16 != 0 {
                    s.sprite_name = ctx
                        .dictionary
                        .get_sprite_name_from_id(b.get_int()?, ctx.sprites)
                        .map(str::to_string);
                } else if flags & 32 != 0 {
                    s.sprite_name = Some(read_string(b)?);
                }
            }
            if header.has_flags(8)? {
                let r = unpack_byte_to_float_unit(b.get()?);
                let g = unpack_byte_to_float_unit(b.get()?);
                let bl = unpack_byte_to_float_unit(b.get()?);
                s.custom_color = Some([r, g, bl]);
            }
            s.do_not_sync = header.has_flags(16)?;
            self.outline_on_mouseover = header.has_flags(32)?;
            if header.has_flags(64)? {
                self.load_extra(b, world_version, ctx, &mut s)?;
            }
        }
        if self.sprite.is_none() {
            self.sprite = Some(ctx.sprites.create_sprite(s.sprite_name.as_deref()));
        }
        self.load_class(b, world_version, ctx, square_room, &mut s)?;
        self.save = Some(Box::new(s));
        Ok(())
    }

    /// zombie.iso.IsoObject.load, the part under header flag 64 (a short BitHeader of optional state).
    fn load_extra<R: JavaRandom>(
        &mut self,
        b: &mut ByteBuffer,
        world_version: i32,
        ctx: &mut SaveLoadContext<R>,
        s: &mut ObjectSave,
    ) -> Result<(), String> {
        let bits = BitHeaderRead::alloc_read(HeaderSize::Short, b)?;
        if bits.has_flags(1)? {
            let count = b.get()?;
            for _ in 0..count.max(0) {
                // IsoWallBloodSplat.load
                let world_age = b.get_float()?;
                let sprite = ctx.sprites.get_sprite_by_id(b.get_int()?);
                s.wall_blood_splats.push(WallBloodSplat {
                    world_age: Some(world_age),
                    sprite,
                });
            }
        }
        if bits.has_flags(2)? {
            let n = b.get()?;
            for _ in 0..n.max(0) {
                let mut c = ItemContainer {
                    capacity: 50,
                    ..ItemContainer::default()
                };
                c.load(b, ctx.items)?;
                s.containers.push(c);
            }
        }
        if bits.has_flags(4)? {
            s.mod_data = Some(KahluaTable::load(b, world_version)?);
        }
        s.special_tooltip = bits.has_flags(8)?;
        if bits.has_flags(16)? {
            s.key_id = b.get_int()?;
        }
        s.uses_external_water_source = bits.has_flags(32)?;
        if bits.has_flags(64)? {
            s.sheet_rope_health = Some(b.get_float()?);
        }
        if bits.has_flags(128)? {
            self.render_y_offset = b.get_float()?;
        }
        if bits.has_flags(256)? {
            let name = if bits.has_flags(512)? {
                Some(read_string(b)?)
            } else {
                ctx.dictionary
                    .get_sprite_name_from_id(b.get_int()?, ctx.sprites)
                    .map(str::to_string)
            };
            if let Some(name) = name.filter(|n| !n.is_empty()) {
                let spr = ctx.sprites.get_sprite(&name);
                ctx.sprites.sprite_mut(spr).name = Some(name);
                s.overlay_sprite = Some(spr);
            }
        }
        if bits.has_flags(1024)? {
            let c = [b.get()?, b.get()?, b.get()?, b.get()?].map(unpack_byte_to_float_unit);
            if s.overlay_sprite.is_some() {
                s.overlay_sprite_color = Some(c);
            }
        }
        s.moved_thumpable = bits.has_flags(2048)?;
        if bits.has_flags(4096)? {
            s.components = load_entity(b)?;
        }
        if bits.has_flags(8192)? {
            s.sprite_model_name = Some(read_string(b)?);
        }
        Ok(())
    }

    /// The `load` of the zombie.iso.objects class after `super.load`.
    fn load_class<R: JavaRandom>(
        &mut self,
        b: &mut ByteBuffer,
        world_version: i32,
        ctx: &mut SaveLoadContext<R>,
        square_room: Option<usize>,
        s: &mut ObjectSave,
    ) -> Result<(), String> {
        let sprite = self.sprite;
        s.class = match &mut self.kind {
            // zombie.iso.objects.IsoCurtain.load
            ObjectKind::IsoCurtain(c) => {
                c.open = b.get()? != 0;
                c.north = b.get()? != 0;
                let health = b.get_int()?;
                let barricade_strength = b.get_int()?;
                let other = ctx.sprites.get_sprite_by_id(b.get_int()?);
                if c.open {
                    c.closed_sprite = other;
                    c.open_sprite = sprite;
                } else {
                    c.open_sprite = other;
                    c.closed_sprite = sprite;
                }
                ClassSave::Curtain {
                    health,
                    barricade_strength,
                }
            }
            // zombie.iso.objects.IsoDoor.load
            ObjectKind::IsoDoor(d) => {
                d.open = b.get()? != 0;
                d.locked = b.get()? != 0;
                d.north = b.get()? != 0;
                let health = b.get_int()?;
                let max_health = b.get_int()?;
                d.closed_sprite = ctx.sprites.get_sprite_by_id(b.get_int()?);
                d.open_sprite = ctx.sprites.get_sprite_by_id(b.get_int()?);
                self.outline_on_mouseover = true;
                let key_id = b.get_int()?;
                d.locked_by_key = b.get()? != 0;
                let f = i32::from(b.get()?);
                let has_curtain = f & 1 != 0;
                ClassSave::Door {
                    health,
                    max_health,
                    key_id,
                    has_curtain,
                    curtain_open: has_curtain && f & 2 != 0,
                    curtain_inside: has_curtain && f & 4 != 0,
                }
            }
            // zombie.iso.objects.IsoWindow.load
            ObjectKind::IsoWindow(w) => {
                w.open = b.get()? != 0;
                w.north = b.get()? != 0;
                let health = b.get_int()?;
                w.locked = b.get()? != 0;
                w.perma_locked = b.get()? != 0;
                w.smashed = b.get()? != 0;
                w.glass_removed = b.get()? != 0;
                if let Some(s) = optional_sprite(b, ctx.sprites)? {
                    w.open_sprite = s;
                }
                if let Some(s) = optional_sprite(b, ctx.sprites)? {
                    w.closed_sprite = s;
                }
                if let Some(s) = optional_sprite(b, ctx.sprites)? {
                    w.smashed_sprite = s;
                }
                if let Some(s) = optional_sprite(b, ctx.sprites)? {
                    w.glass_removed_sprite = s;
                }
                let max_health = b.get_int()?;
                window_sprites(
                    ctx.sprites,
                    w.north,
                    w.closed_sprite,
                    [w.smashed_sprite, w.open_sprite, w.glass_removed_sprite],
                );
                ClassSave::Window { health, max_health }
            }
            // zombie.iso.objects.IsoWindowFrame.load
            ObjectKind::IsoWindowFrame { north } => {
                *north = b.get()? != 0;
                ClassSave::None
            }
            // zombie.iso.objects.IsoTree.load
            ObjectKind::IsoTree(t) => {
                t.log_yield = i32::from(b.get()?);
                t.damage = i32::from(b.get()?) * 10;
                if let Some(v) =
                    sprite.and_then(|s| ctx.sprites.sprite(s).properties.get(ctx.alias, "tree"))
                {
                    t.size = java::parse_int(v).ok_or_else(|| nfe(v))?.clamp(1, 8);
                }
                ClassSave::None
            }
            // zombie.iso.objects.IsoLightSwitch.load
            ObjectKind::IsoLightSwitch => ClassSave::LightSwitch(load_light_switch(
                b,
                world_version,
                ctx,
                sprite,
                square_room,
            )?),
            // zombie.iso.objects.IsoBarbecue.load
            ObjectKind::IsoBarbecue {
                has_propane_tank,
                fuel_amount_minutes,
            } => {
                *has_propane_tank = b.get()? != 0;
                *fuel_amount_minutes = b.get_int()?;
                let lit = b.get()? != 0;
                let last_update_time = b.get_float()?;
                let minutes_since_extinguished = b.get_int()?;
                let normal = optional_sprite(b, ctx.sprites)?.flatten();
                let no_tank = optional_sprite(b, ctx.sprites)?.flatten();
                ClassSave::Barbecue {
                    lit,
                    last_update_time,
                    minutes_since_extinguished,
                    normal,
                    no_tank,
                }
            }
            // zombie.iso.objects.IsoFireplace.load
            ObjectKind::IsoFireplace => ClassSave::Fireplace {
                fuel_amount_minutes: b.get_int()?,
                lit: b.get()? != 0,
                last_update_time: b.get_float()?,
                minutes_since_extinguished: b.get_int()?,
            },
            // zombie.iso.objects.IsoStove.load
            ObjectKind::IsoStove => ClassSave::Stove {
                activated: b.get()? != 0,
                seconds_timer: b.get_int()?,
                max_temperature: b.get_float()?,
                first_turn_on: b.get()? != 0,
                broken: b.get()? != 0,
            },
            // zombie.iso.objects.IsoClothingWasher.load -> ClothingWasherLogic.load
            ObjectKind::IsoClothingWasher => ClassSave::Washer {
                activated: b.get()? != 0,
                last_update: b.get_float()?,
            },
            // zombie.iso.objects.IsoClothingDryer.load -> ClothingDryerLogic.load
            ObjectKind::IsoClothingDryer => ClassSave::Dryer {
                activated: b.get()? != 0,
            },
            // zombie.iso.objects.IsoCombinationWasherDryer.load
            ObjectKind::IsoCombinationWasherDryer => ClassSave::WasherDryer {
                dryer: b.get()? != 0,
                washer_activated: b.get()? != 0,
                washer_last_update: b.get_float()?,
                dryer_activated: b.get()? != 0,
            },
            // zombie.iso.objects.IsoWaveSignal.load (IsoRadio, IsoTelevision; IsoTelevision.load then drops the
            // overlay sprite)
            ObjectKind::IsoRadio | ObjectKind::IsoTelevision => {
                let d = if b.get()? != 0 {
                    Some(DeviceData::load(b)?)
                } else {
                    None
                };
                if matches!(self.kind, ObjectKind::IsoTelevision) {
                    s.overlay_sprite = None;
                }
                ClassSave::WaveSignal(d)
            }
            // zombie.iso.objects.IsoMannequin.load
            ObjectKind::IsoMannequin => ClassSave::Mannequin(Box::new(load_mannequin(b, ctx, s)?)),
            ObjectKind::IsoJukebox => return Err("loading saved jukeboxes is not ported".into()),
            ObjectKind::IsoObject
            | ObjectKind::IsoBrokenGlass
            | ObjectKind::IsoWorldInventoryObject => ClassSave::None,
        };
        Ok(())
    }

    /// zombie.iso.objects.IsoWorldInventoryObject.load (does not call IsoObject.load). On a dedicated server the
    /// object's sprite is an unnamed sprite without a texture.
    fn load_world_inventory_object<R: JavaRandom>(
        &mut self,
        b: &mut ByteBuffer,
        ctx: &mut SaveLoadContext<R>,
    ) -> Result<(), String> {
        let (xoff, yoff, zoff) = (b.get_float()?, b.get_float()?, b.get_float()?);
        let offset_x = b.get_float()?;
        let offset_y = b.get_float()?;
        self.sprite = Some(ctx.sprites.create_sprite(None));
        let item = InventoryItem::load_item(b, ctx.items)?;
        let mut w = WorldInventorySave {
            xoff,
            yoff,
            zoff,
            offset_x,
            offset_y,
            item,
            drop_time: 0.0,
            ignore_remove_sandbox: false,
            extended_placement: false,
        };
        let mut components = Vec::new();
        if w.item.is_none() {
            b.get_double()?;
            BitHeaderRead::alloc_read(HeaderSize::Byte, b)?;
        } else {
            w.drop_time = b.get_double()?;
            let bits = BitHeaderRead::alloc_read(HeaderSize::Byte, b)?;
            w.ignore_remove_sandbox = bits.has_flags(1)?;
            if bits.has_flags(2)? {
                components = load_entity(b)?;
            }
            w.extended_placement = bits.has_flags(4)?;
        }
        self.save = Some(Box::new(ObjectSave {
            components,
            class: ClassSave::WorldInventory(Box::new(w)),
            ..ObjectSave::default()
        }));
        Ok(())
    }
}

/// zombie.iso.objects.IsoLightSwitch.load after `super.load`. The light source it creates is not modelled.
fn load_light_switch<R: JavaRandom>(
    b: &mut ByteBuffer,
    world_version: i32,
    ctx: &mut SaveLoadContext<R>,
    sprite: Option<SpriteRef>,
    square_room: Option<usize>,
) -> Result<LightSwitchSave, String> {
    let mut l = LightSwitchSave {
        light_room: b.get()? != 0,
        ..LightSwitchSave::default()
    };
    if world_version < 206 {
        return Err("light switches of world versions before 206 are not ported".into());
    }
    l.room_id = b.get_long()?;
    l.activated = b.get()? != 0;
    l.can_be_modified = b.get()? != 0;
    if l.can_be_modified {
        l.use_battery = b.get()? != 0;
        l.has_battery = b.get()? != 0;
        l.bulb_item = if b.get()? != 0 {
            Some(read_string(b)?)
        } else {
            None
        };
        l.power = b.get_float()?;
        l.delta = b.get_float()?;
        l.primary = [b.get_float()?, b.get_float()?, b.get_float()?];
    }
    l.last_minute_stamp = b.get_long()?;
    l.bulb_burn_minutes = b.get_int()?;
    let props = sprite.map(|s| &ctx.sprites.sprite(s).properties);
    l.street_light = props.is_some_and(|p| p.has_str(ctx.alias, "streetlight"));
    match square_room {
        Some(room) if l.light_room => {
            l.activated = ctx
                .meta_grid
                .room(ctx.meta_grid.iso_rooms[room].def)
                .lights_active;
            l.activated_from_room = true;
        }
        _ => {
            if let Some(p) = props.filter(|p| p.has_str(ctx.alias, pt::RED_LIGHT)) {
                if !l.can_be_modified {
                    let f = |name: &str| -> Result<f32, String> {
                        let v = p.get(ctx.alias, name).unwrap_or("null");
                        Ok(parse_float(v).ok_or_else(|| nfe(v))? / 255.0)
                    };
                    l.primary = [f(pt::RED_LIGHT)?, f(pt::GREEN_LIGHT)?, f(pt::BLUE_LIGHT)?];
                }
            }
        }
    }
    Ok(l)
}

/// zombie.iso.objects.IsoMannequin.load after `super.load`: the mannequin's container is the object's first container
/// (a new explored "mannequin" container if the object part had none), emptied and then loaded when saved.
fn load_mannequin<R: JavaRandom>(
    b: &mut ByteBuffer,
    ctx: &mut SaveLoadContext<R>,
    s: &mut ObjectSave,
) -> Result<MannequinSave, String> {
    let dir = i32::from(b.get()?);
    let init = b.get()? != 0;
    let female = b.get()? != 0;
    let zombie = b.get()? != 0;
    let skeleton = b.get()? != 0;
    let mannequin_script_name = read_string(b)?;
    let pose = read_string(b)?;
    let human_visual = HumanVisual::load(b)?;
    if s.containers.is_empty() {
        s.containers.push(ItemContainer {
            r#type: "mannequin".into(),
            explored: true,
            capacity: 50,
            ..ItemContainer::default()
        });
    }
    let c = &mut s.containers[0];
    c.items.clear();
    let mut worn_items = Vec::new();
    if b.get()? != 0 {
        c.id = b.get_int()?;
        c.load(b, ctx.items)?;
        let n = b.get()?;
        for _ in 0..n.max(0) {
            let location = read_string(b)?;
            worn_items.push((location, b.get_short()?));
        }
    }
    Ok(MannequinSave {
        dir,
        init,
        female,
        zombie,
        skeleton,
        mannequin_script_name,
        pose,
        human_visual,
        worn_items,
    })
}

impl IsoGridSquare {
    /// zombie.iso.IsoGridSquare.load (ByteBuffer, int, false). World items are kept whatever their drop time (the
    /// sandbox's world-item removal is not applied); items whose script is obsolete are dropped as in the original.
    pub fn load<R: JavaRandom>(
        &mut self,
        b: &mut ByteBuffer,
        world_version: i32,
        ctx: &mut SaveLoadContext<R>,
    ) -> Result<(), String> {
        let mut s = SquareSave {
            erosion: erosion_data::Square::load(b)?,
            ..SquareSave::default()
        };
        let header = BitHeaderRead::alloc_read(HeaderSize::Byte, b)?;
        if !header.equals(0) {
            if header.has_flags(1)? {
                let objs = if header.has_flags(2)? {
                    2
                } else if header.has_flags(4)? {
                    3
                } else if header.has_flags(8)? {
                    i32::from(b.get_short()?)
                } else {
                    1
                };
                for _ in 0..objs {
                    let flags_obj = i32::from(b.get()?);
                    let b_special = flags_obj & 2 != 0;
                    let b_world = flags_obj & 4 != 0;
                    let Some(kind) = IsoObject::factory_from_file_input(b)? else {
                        continue;
                    };
                    let mut obj = IsoObject::new(kind, None);
                    let room = if self.room_id == -1 { None } else { self.room };
                    obj.load(b, world_version, ctx, room)?;
                    if let Some(ObjectSave {
                        class: ClassSave::WorldInventory(w),
                        ..
                    }) = obj.save.as_deref()
                    {
                        let Some(item) = w.item.as_ref() else {
                            continue;
                        };
                        if ctx.items.is_obsolete(&item.full_type) {
                            continue;
                        }
                    }
                    let dropped_window = matches!(obj.kind, ObjectKind::IsoWindow(_))
                        && obj
                            .sprite
                            .and_then(|sp| ctx.sprites.sprite(sp).name.as_deref())
                            .is_some_and(|n| {
                                n == "walls_special_01_8" || n == "walls_special_01_9"
                            });
                    if !dropped_window {
                        obj.special = b_special;
                        if b_world {
                            s.world_objects.push(self.objects.len());
                        }
                        self.objects.push(obj);
                    }
                }
            }
            s.overlay_done = header.has_flags(16)?;
            self.have_roof = header.has_flags(32)?;
            if header.has_flags(64)? {
                let bits = BitHeaderRead::alloc_read(HeaderSize::Byte, b)?;
                if bits.has_flags(1)? {
                    let n = b.get_short()?;
                    for _ in 0..n.max(0) {
                        if b.get()? == 0 {
                            continue;
                        }
                        let class_id = b.get()?;
                        if class_id != 11 {
                            return Err(format!("loading static moving objects of class id {class_id} is not ported"));
                        }
                        s.static_moving_objects.push(IsoDeadBody::load(
                            b,
                            world_version,
                            ctx.items,
                        )?);
                    }
                }
                if bits.has_flags(2)? {
                    s.mod_data = Some(KahluaTable::load(b, world_version)?);
                }
                s.burnt_out = bits.has_flags(4)?;
                if bits.has_flags(8)? {
                    // trap position x, y, z: read and dropped
                    b.get_int()?;
                    b.get_int()?;
                    b.get_int()?;
                }
                s.have_sheet_rope = bits.has_flags(16)?;
            }
        }
        s.vis = b.get()?;
        self.save = Some(Box::new(s));
        Ok(())
    }
}

impl IsoChunk {
    /// zombie.iso.IsoChunk.LoadFromDiskOrBuffer: the chunk, or `Err` for a "black chunk" (no square at (0,0,0) and
    /// none at (7,7,0)).
    pub fn load_from_disk_or_buffer<R: JavaRandom>(
        data: &[u8],
        wx: i32,
        wy: i32,
        ctx: &mut SaveLoadContext<R>,
    ) -> Result<IsoChunk, String> {
        let ch = Self::load_from_buffer(data, wx, wy, ctx)?;
        if ch.get_grid_square(0, 0, 0).is_none() && ch.get_grid_square(7, 7, 0).is_none() {
            return Err(format!("black chunk {wx},{wy}"));
        }
        Ok(ch)
    }

    /// zombie.iso.IsoChunk.LoadFromDiskOrBufferInternal: a whole chunk save. Length and CRC mismatches are recorded,
    /// as the original's sanity check logs them. Saved vehicles are not ported (`Err` if the chunk has any).
    pub fn load_from_buffer<R: JavaRandom>(
        data: &[u8],
        wx: i32,
        wy: i32,
        ctx: &mut SaveLoadContext<R>,
    ) -> Result<IsoChunk, String> {
        let mut b = ByteBuffer::wrap(data);
        if b.get()? != 0 {
            return Err("debug saves are not ported".into());
        }
        let world_version = b.get_int()?;
        if world_version > WORLD_VERSION {
            return Err(format!(
                "unknown world version {world_version} while reading chunk {wx},{wy}"
            ));
        }
        let len = b.get_int()?;
        let crc = b.get_long()?;
        let mut c = ChunkSave {
            world_version,
            length_mismatch: i64::from(len) != data.len() as i64,
            crc_mismatch: crc != i64::from(java::crc32(data.get(17..).unwrap_or(&[]))),
            ..ChunkSave::default()
        };
        if world_version >= 209 {
            c.blending_done_full = b.get()? != 0;
        }
        if world_version >= 210 {
            c.blending_modified = read_flags::<4>(&mut b)?;
            c.blending_done_partial = b.get()? != 0;
            if c.blending_modified != [false; 4] && c.blending_done_partial {
                for d in &mut c.blending_depth {
                    *d = b.get()?;
                }
            }
        }
        if world_version >= 214 {
            c.attachments_done_full = b.get()? != 0;
            c.attachments_state = read_flags::<5>(&mut b)?;
        }
        if world_version >= 221 {
            let count = b.get_short()?;
            if count != 0 {
                let mut v = Vec::new();
                for _ in 0..count {
                    // SquareCoord.load
                    v.push((b.get_int()?, b.get_int()?, i32::from(b.get()?)));
                }
                c.attachments_partial = Some(v);
            }
        }
        let (max_level, min_level) = if world_version >= 206 {
            (b.get_int()?, b.get_int()?)
        } else {
            (7, 0)
        };
        let mut ch = IsoChunk::new(wx, wy);
        ch.set_min_max_level(min_level, max_level);
        let n = b.get_int()?;
        for _ in 0..n {
            // IsoFloorBloodSplat.load
            let x = (i32::from(b.get()?) & 255) as f32 / 255.0 * 8.0;
            let y = (i32::from(b.get()?) & 255) as f32 / 255.0 * 8.0;
            let z = (i32::from(b.get()?) & 255) as f32 / 255.0 * 8.0;
            let r#type = i32::from(b.get()?);
            let world_age = b.get_float()?;
            let index = i32::from(b.get()?);
            c.floor_blood_splats.push(FloorBloodSplat {
                x,
                y,
                z,
                r#type,
                world_age,
                index,
            });
        }
        for x in 0..8 {
            for y in 0..8 {
                let flags = if world_version >= 206 {
                    b.get_long()?
                } else {
                    i64::from(b.get()?) << 32
                };
                for zz in min_level..=max_level {
                    if flags & (1i64 << (zz + 32)) == 0 {
                        continue;
                    }
                    let mut gs = IsoGridSquare::new(x + wx * 8, y + wy * 8, zz);
                    if ctx.has_lot_header {
                        // setRoomID: the room (and its building) is created on first contact.
                        let room_id = ctx
                            .meta_grid
                            .get_room_at(gs.x, gs.y, gs.z)
                            .map_or(-1, |r| ctx.meta_grid.room(r).id);
                        gs.room_id = room_id;
                        if room_id != -1 {
                            let rand = &mut *ctx.rand;
                            gs.room = ctx.meta_grid.get_room_by_id(room_id, &mut |n| rand.next(n));
                        }
                        if let Some(id) = ctx
                            .meta_grid
                            .get_empty_outside_at(gs.x, gs.y, gs.z)
                            .map(|r| ctx.meta_grid.room(r).id)
                        {
                            let rand = &mut *ctx.rand;
                            gs.roof_hide_building = ctx
                                .meta_grid
                                .get_room_by_id(id, &mut |n| rand.next(n))
                                .map(|r| ctx.meta_grid.iso_rooms[r].building);
                        }
                    }
                    gs.load(&mut b, world_version, ctx)?;
                    gs.fix_stackable_objects(ctx.sprites, ctx.alias);
                    ch.set_square(x, y, zz, Some(gs));
                }
            }
        }
        c.erosion = erosion_data::Chunk::load(&mut b)?;
        let count = b.get_short()?;
        if count > 0 {
            let mut v = Vec::new();
            for _ in 0..count {
                v.push((b.get_int()?, b.get_int()?, i32::from(b.get()?)));
            }
            c.generators_touching_this_chunk = Some(v);
        }
        if b.get_short()? > 0 {
            return Err(format!(
                "chunk {wx},{wy}: loading saved vehicles is not ported"
            ));
        }
        c.loot_respawn_hour = b.get_int()?;
        if world_version < 206 {
            return Err("spawned rooms of world versions before 206 are not ported".into());
        }
        let rooms = b.get_short()?;
        for _ in 0..rooms.max(0) {
            // addSpawnedRoom
            let id = b.get_long()?;
            if !c.spawned_rooms.contains(&id) {
                c.spawned_rooms.push(id);
            }
        }
        ch.save = Some(Box::new(c));
        Ok(ch)
    }
}
