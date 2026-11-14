//! zombie.inventory.InventoryItem: an item as a save stores it. `loadItem` frames it; `InventoryItem.load` and the
//! subclass loads of zombie.inventory.types read it into the state the item factory made from its script. Only the
//! state that `load` reads or resets is modelled; what stays as the script made it is not.
use super::types::alarm_clock::AlarmClockSave;
use super::types::clothing::ClothingSave;
use super::types::food::FoodSave;
use super::types::hand_weapon::HandWeaponSave;
use super::types::literature::LiteratureSave;
use super::types::moveable::MoveableSave;
use super::types::{InventoryContainerSave, KeySave, MapItemSave, RadioSave};
use crate::java_nio::{
    read_string, unpack_byte_to_float_unit, BitHeaderRead, ByteBuffer, HeaderSize,
};
use crate::se::krka::kahlua::j2se::kahlua_table_impl::{KahluaTable, LuaValue};
use crate::zombie::core::math::pz_math;
use crate::zombie::core::skinnedmodel::visual::item_visual::ItemVisual;
use crate::zombie::iso::chunk_save::load_entity;

/// zombie.scripting.objects.ItemType: the item script's type, which picks the class `Item.InstanceItem` creates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemType {
    AlarmClock,
    AlarmClockClothing,
    Animal,
    Clothing,
    Container,
    Drainable,
    Food,
    Key,
    KeyRing,
    Literature,
    Map,
    Moveable,
    Normal,
    Radio,
    Weapon,
    WeaponPart,
}

impl ItemType {
    /// zombie.scripting.objects.ItemType.get(ResourceLocation.of(val)) for the base types: `namespace:path` (namespace
    /// `base` when absent), both lower-cased.
    pub fn from_script(val: &str) -> Option<Self> {
        let (namespace, path) = val.split_once(':').unwrap_or(("base", val));
        if namespace.to_lowercase() != "base" {
            return None;
        }
        Some(match path.to_lowercase().as_str() {
            "alarmclock" => ItemType::AlarmClock,
            "alarmclockclothing" => ItemType::AlarmClockClothing,
            "animal" => ItemType::Animal,
            "clothing" => ItemType::Clothing,
            "container" => ItemType::Container,
            "drainable" => ItemType::Drainable,
            "food" => ItemType::Food,
            "key" => ItemType::Key,
            "keyring" => ItemType::KeyRing,
            "literature" => ItemType::Literature,
            "map" => ItemType::Map,
            "moveable" => ItemType::Moveable,
            "normal" => ItemType::Normal,
            "radio" => ItemType::Radio,
            "weapon" => ItemType::Weapon,
            "weaponpart" => ItemType::WeaponPart,
            _ => return None,
        })
    }

    /// The simple name of the class zombie.scripting.objects.Item.InstanceItem creates for this type (`None`: no
    /// branch, as for KeyRing).
    pub fn class_name(self) -> Option<&'static str> {
        Some(match self {
            ItemType::AlarmClock => "AlarmClock",
            ItemType::AlarmClockClothing => "AlarmClockClothing",
            ItemType::Animal => "AnimalInventoryItem",
            ItemType::Clothing => "Clothing",
            ItemType::Container => "InventoryContainer",
            ItemType::Drainable => "DrainableComboItem",
            ItemType::Food => "Food",
            ItemType::Key => "Key",
            ItemType::KeyRing => return None,
            ItemType::Literature => "Literature",
            ItemType::Map => "MapItem",
            ItemType::Moveable => "Moveable",
            ItemType::Normal => "ComboItem",
            ItemType::Radio => "Radio",
            ItemType::Weapon => "HandWeapon",
            ItemType::WeaponPart => "WeaponPart",
        })
    }
}

/// What `InventoryItemFactory.CreateItem(short)` gives `load` to work on: the dictionary's full type and, from the
/// item script, the class, `conditionMax` and (food) `baseHunger`.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemTemplate {
    pub full_type: String,
    pub item_type: ItemType,
    pub condition_max: i32,
    /// Food's `baseHunger` as Item.InstanceItem sets it (`hungerChange / 100`); 0 for other classes.
    pub base_hunger: f32,
}

/// What item loading needs from the running game.
pub trait ItemFactory {
    /// zombie.inventory.InventoryItemFactory.CreateItem (short): `None` where it returns null (unknown id, invalid
    /// entry or no such script).
    fn create_item(&self, registry_id: i16) -> Option<ItemTemplate>;
    /// `ScriptManager.instance.FindItem(fullType)` exists and `getObsolete()`.
    fn is_obsolete(&self, full_type: &str) -> bool;
    /// `ZomboidRadio.getInstance().getRecordedMedia().getMediaDataFromIndex(index)`: the media id, `None` for null.
    fn media_data_from_index(&self, index: i16) -> Option<String>;
}

/// Errors of features this port does not read yet: unlike the original's exceptions (which make the item null) they
/// stop the load, so that a gap shows up instead of a silently missing item.
pub fn not_ported(what: &str) -> String {
    format!("not ported: {what}")
}

fn is_not_ported(e: &str) -> bool {
    e.starts_with("not ported: ")
}

/// The subclass part of an item.
#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    /// ComboItem, DrainableComboItem and WeaponPart, which have no load of their own.
    Plain,
    Key(KeySave),
    AlarmClock(AlarmClockSave),
    /// Clothing, and AlarmClockClothing with its alarm.
    Clothing(ClothingSave, Option<AlarmClockSave>),
    Container(Box<InventoryContainerSave>),
    Food(Box<FoodSave>),
    Literature(LiteratureSave),
    Map(MapItemSave),
    Moveable(MoveableSave),
    Radio(MoveableSave, RadioSave),
    Weapon(Box<HandWeaponSave>),
}

/// zombie.inventory.InventoryItem, the state `load` reads or resets.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryItem {
    pub registry_id: i16,
    /// `WorldDictionary.getItemTypeFromID(registry_id)`.
    pub full_type: String,
    pub item_type: ItemType,
    pub condition_max: i32,
    /// See `ItemTemplate::base_hunger`.
    pub base_hunger: f32,
    /// The byte `save()` writes after the registry id (always -1).
    pub save_type: i8,
    pub id: i32,
    pub uses: i32,
    pub condition: i32,
    /// Set where the save holds a condition (`setConditionWhileLoading`).
    pub broken: Option<bool>,
    pub visual: Option<ItemVisual>,
    pub custom_color: bool,
    /// `col` (r, g, b, a).
    pub col: [f32; 4],
    pub item_capacity: f32,
    /// `table` (modData) as loaded, without the `customName` entry (see `custom_name_entry`).
    pub table: Option<KahluaTable>,
    /// The value `setCustomName` rawsets as modData `customName`: `Some(None)` is `originalName` (the item script's
    /// translated display name, not modelled).
    pub custom_name_entry: Option<Option<String>>,
    pub activated: bool,
    pub have_been_repaired: i32,
    /// `None` is `originalName`.
    pub name: Option<String>,
    pub byte_data: Option<Vec<u8>>,
    /// Registry ids of `extraItems` (field initialiser: an empty list).
    pub extra_items: Vec<i16>,
    pub custom_name: bool,
    /// `actualWeight` where the save sets a custom weight.
    pub custom_weight: Option<f32>,
    pub key_id: i32,
    pub remote_control_id: i32,
    pub remote_range: i32,
    pub color_red: f32,
    pub color_green: f32,
    pub color_blue: f32,
    pub worker: Option<String>,
    pub wet_cooldown: f32,
    pub favorite: bool,
    pub stash_map: Option<String>,
    pub zombie_infected: bool,
    pub current_ammo_count: i32,
    pub attached_slot: i32,
    pub attached_slot_type: Option<String>,
    pub attached_to_model: Option<String>,
    pub max_capacity: i32,
    pub recorded_media_index: i16,
    /// The media id where `setRecordedMediaIndex` found one; the name then is the media's translated display name
    /// (not modelled).
    pub recorded_media_id: Option<String>,
    pub world_x_rotation: f32,
    pub world_y_rotation: f32,
    pub world_z_rotation: f32,
    pub world_scale: f32,
    pub is_initialised: bool,
    /// `loadEntity`: (component id, bytes), kept raw.
    pub components: Option<Vec<(i16, Vec<u8>)>>,
    /// `setTexture(Texture.getSharedTexture(name))`
    pub texture: Option<String>,
    pub model_index: i32,
    pub kind: ItemKind,
}

impl InventoryItem {
    /// The item `InventoryItemFactory.CreateItem(short)` returns, as far as `load` depends on it: the field
    /// initialisers of InventoryItem (`worldXRotation`/`worldYRotation` 0, `modelIndex` -1).
    pub fn new(registry_id: i16, save_type: i8, t: ItemTemplate) -> Self {
        InventoryItem {
            registry_id,
            full_type: t.full_type,
            item_type: t.item_type,
            condition_max: t.condition_max,
            base_hunger: t.base_hunger,
            save_type,
            id: 0,
            uses: 1,
            condition: t.condition_max,
            broken: None,
            visual: None,
            custom_color: false,
            col: [1.0; 4],
            item_capacity: -1.0,
            table: None,
            custom_name_entry: None,
            activated: false,
            have_been_repaired: 0,
            name: None,
            byte_data: None,
            extra_items: Vec::new(),
            custom_name: false,
            custom_weight: None,
            key_id: -1,
            remote_control_id: -1,
            remote_range: 0,
            color_red: 1.0,
            color_green: 1.0,
            color_blue: 1.0,
            worker: None,
            wet_cooldown: -1.0,
            favorite: false,
            stash_map: None,
            zombie_infected: false,
            current_ammo_count: 0,
            attached_slot: -1,
            attached_slot_type: None,
            attached_to_model: None,
            max_capacity: -1,
            recorded_media_index: -1,
            recorded_media_id: None,
            world_x_rotation: 0.0,
            world_y_rotation: 0.0,
            world_z_rotation: -1.0,
            world_scale: 1.0,
            is_initialised: false,
            components: None,
            texture: None,
            model_index: -1,
            kind: ItemKind::Plain,
        }
    }

    /// zombie.inventory.InventoryItem.loadItem (ByteBuffer, int): an int length, the registry id, the save type, then
    /// the item. `Ok(None)` where the original gets a null item (it cannot create it, or its load throws); the length
    /// tells it where the next item starts either way.
    pub fn load_item(
        b: &mut ByteBuffer,
        world_version: i32,
        factory: &dyn ItemFactory,
    ) -> Result<Option<Self>, String> {
        let data_len = b.get_int()?;
        if data_len <= 0 {
            return Err(format!(
                "java.io.IOException: InventoryItem.loadItem() invalid item data length: {data_len}"
            ));
        }
        let position = b.position();
        let end = position + data_len as usize;
        let registry_id = b.get_short()?;
        let save_type = b.get()?;
        let item = match factory.create_item(registry_id) {
            Some(t) => {
                let mut it = InventoryItem::new(registry_id, save_type, t);
                match it.load(b, world_version, factory) {
                    Ok(()) => Some(it),
                    Err(e) if is_not_ported(&e) => return Err(e),
                    // ExceptionLogger.logException; item = null
                    Err(_) => None,
                }
            }
            None => None,
        };
        if item.is_some() {
            if b.position() != end {
                b.set_position(end)?;
            }
        } else if b.position() >= end {
            b.set_position(end)?;
        } else {
            // `while (position < end) input.get()`: throws past the buffer's limit.
            b.get_bytes(end - b.position())?;
        }
        Ok(item)
    }

    /// The item's `load`: the subclass load for its class (each calls `InventoryItem.load` first).
    pub fn load(
        &mut self,
        b: &mut ByteBuffer,
        world_version: i32,
        factory: &dyn ItemFactory,
    ) -> Result<(), String> {
        match self.item_type {
            ItemType::Animal => return Err(not_ported("AnimalInventoryItem.load")),
            ItemType::KeyRing => {
                return Err(not_ported("Item.InstanceItem for KeyRing (no class)"))
            }
            _ => {}
        }
        self.load_base(b, world_version, factory)?;
        self.kind = match self.item_type {
            ItemType::Normal | ItemType::Drainable | ItemType::WeaponPart => ItemKind::Plain,
            ItemType::Key => {
                let k = KeySave::load(b)?;
                self.key_id = k.key_id.unwrap_or(-1);
                ItemKind::Key(k)
            }
            ItemType::AlarmClock => ItemKind::AlarmClock(AlarmClockSave::load(b, world_version)?),
            ItemType::Clothing => ItemKind::Clothing(ClothingSave::load(b)?, None),
            ItemType::AlarmClockClothing => {
                let c = ClothingSave::load(b)?;
                ItemKind::Clothing(c, Some(AlarmClockSave::load(b, world_version)?))
            }
            ItemType::Container => ItemKind::Container(Box::new(InventoryContainerSave::load(
                b,
                world_version,
                factory,
            )?)),
            ItemType::Food => ItemKind::Food(Box::new(FoodSave::load(b)?)),
            ItemType::Literature => ItemKind::Literature(LiteratureSave::load(b, world_version)?),
            ItemType::Map => ItemKind::Map(MapItemSave::load(b)?),
            ItemType::Moveable => ItemKind::Moveable(MoveableSave::load(b)?),
            ItemType::Radio => {
                let m = MoveableSave::load(b)?;
                ItemKind::Radio(m, RadioSave::load(b)?)
            }
            ItemType::Weapon => {
                ItemKind::Weapon(Box::new(HandWeaponSave::load(b, world_version, factory)?))
            }
            ItemType::Animal | ItemType::KeyRing => ItemKind::Plain,
        };
        Ok(())
    }

    /// `setCurrentUses` as the class overrides it. zombie.inventory.types.Food.setCurrentUses: with a non-zero
    /// `baseHunger` it calls `consumeHunger` instead (which scales food values that Food.load then resets) and `uses`
    /// stays. DrainableComboItem's override also updates the weight (not modelled).
    fn set_current_uses(&mut self, newuses: i32) {
        if self.item_type == ItemType::Food {
            if self.base_hunger == 0.0 {
                self.uses = newuses.max(0);
            }
        } else {
            self.uses = newuses;
        }
    }

    /// zombie.inventory.InventoryItem.load (ByteBuffer, int): the id, a byte flag header with the common fields and,
    /// under flag 64, an int flag header with the rest. `synchWithVisual` at the end (model index draws, icon and
    /// tint from the clothing item) is not modelled.
    fn load_base(
        &mut self,
        b: &mut ByteBuffer,
        world_version: i32,
        factory: &dyn ItemFactory,
    ) -> Result<(), String> {
        self.id = b.get_int()?;
        let header = BitHeaderRead::alloc_read(HeaderSize::Byte, b)?;
        self.set_current_uses(1);
        self.name = None;
        self.condition = self.condition_max;
        self.custom_color = false;
        self.col = [1.0; 4];
        self.item_capacity = -1.0;
        self.table = None;
        self.activated = false;
        self.have_been_repaired = 0;
        self.custom_name = false;
        self.custom_weight = None;
        self.key_id = -1;
        self.remote_control_id = -1;
        self.remote_range = 0;
        self.color_red = 1.0;
        self.color_green = 1.0;
        self.color_blue = 1.0;
        self.worker = None;
        self.wet_cooldown = -1.0;
        self.favorite = false;
        self.stash_map = None;
        self.zombie_infected = false;
        self.current_ammo_count = 0;
        self.attached_slot = -1;
        self.attached_slot_type = None;
        self.attached_to_model = None;
        self.max_capacity = -1;
        self.recorded_media_index = -1;
        self.recorded_media_id = None;
        self.world_z_rotation = -1.0;
        self.world_scale = 1.0;
        self.is_initialised = false;
        if header.equals(0) {
            return Ok(());
        }
        if world_version >= 220 {
            if header.has_flags(1)? {
                let uses = b.get_int()?;
                self.set_current_uses(uses);
            }
        } else {
            if header.has_flags(1)? {
                let uses = i32::from(b.get_short()?);
                self.set_current_uses(uses);
            }
            if header.has_flags(2)? {
                b.get()?;
            }
        }
        if header.has_flags(4)? {
            // setConditionWhileLoading
            self.condition = pz_math::clamp_i32(i32::from(b.get()?), 0, self.condition_max);
            self.broken = Some(self.condition <= 0);
        }
        if header.has_flags(8)? {
            self.visual = Some(ItemVisual::load(b)?);
        }
        if header.has_flags(16)? {
            let mut c = [0.0f32; 4];
            for v in &mut c {
                // new Color(r, g, b, a) clamps to [0, 1]; the unpacked bytes already are.
                *v = unpack_byte_to_float_unit(b.get()?).clamp(0.0, 1.0);
            }
            self.col = c;
            self.custom_color = true;
        }
        if header.has_flags(32)? {
            self.item_capacity = b.get_float()?;
        }
        if header.has_flags(64)? {
            let bits = BitHeaderRead::alloc_read(HeaderSize::Integer, b)?;
            if bits.has_flags(1)? {
                self.table = Some(KahluaTable::load(b, world_version)?);
            }
            self.activated = bits.has_flags(2)?;
            if bits.has_flags(4)? {
                self.have_been_repaired = i32::from(b.get_short()?);
            }
            if bits.has_flags(8)? {
                self.name = Some(read_string(b)?);
            }
            if bits.has_flags(16)? {
                let size = b.get_int()?;
                self.byte_data = Some(b.get_bytes(size.max(0) as usize)?.to_vec());
            }
            if bits.has_flags(32)? {
                let it = b.get_int()?;
                if it > 0 {
                    self.extra_items.clear();
                    for _ in 0..it {
                        self.extra_items.push(b.get_short()?);
                    }
                }
            }
            // setCustomName: getModData().rawset("customName", String.valueOf(name)).
            self.custom_name = bits.has_flags(64)?;
            let table = self.table.get_or_insert_with(KahluaTable::default);
            table
                .entries
                .retain(|(k, _)| *k != LuaValue::String("customName".into()));
            self.custom_name_entry = Some(self.name.clone());
            if bits.has_flags(128)? {
                let weight = b.get_float()?;
                if weight >= 0.0 {
                    self.custom_weight = Some(weight);
                }
            }
            if bits.has_flags(256)? {
                self.key_id = b.get_int()?;
            }
            if bits.has_flags(1024)? {
                self.remote_control_id = b.get_int()?;
                self.remote_range = b.get_int()?;
            }
            if bits.has_flags(2048)? {
                self.color_red = unpack_byte_to_float_unit(b.get()?);
                self.color_green = unpack_byte_to_float_unit(b.get()?);
                self.color_blue = unpack_byte_to_float_unit(b.get()?);
                // new Color(r, g, b): no clamping, alpha 1.
                self.col = [self.color_red, self.color_green, self.color_blue, 1.0];
                self.custom_color = true;
            }
            if bits.has_flags(4096)? {
                self.worker = Some(read_string(b)?);
            }
            if bits.has_flags(8192)? {
                self.wet_cooldown = b.get_float()?;
            }
            self.favorite = bits.has_flags(16384)?;
            if bits.has_flags(32768)? {
                self.stash_map = Some(read_string(b)?);
            }
            self.zombie_infected = bits.has_flags(65536)?;
            if bits.has_flags(131072)? {
                self.current_ammo_count = b.get_int()?;
            }
            if bits.has_flags(262144)? {
                self.attached_slot = b.get_int()?;
            }
            if bits.has_flags(524288)? {
                self.attached_slot_type = Some(read_string(b)?);
            }
            if bits.has_flags(1048576)? {
                self.attached_to_model = Some(read_string(b)?);
            }
            if bits.has_flags(2097152)? {
                self.max_capacity = b.get_int()?;
            }
            if bits.has_flags(4194304)? {
                // setRecordedMediaIndex
                self.recorded_media_index = b.get_short()?;
                if self.recorded_media_index >= 0 {
                    match factory.media_data_from_index(self.recorded_media_index) {
                        Some(id) => self.recorded_media_id = Some(id),
                        None => self.recorded_media_index = -1,
                    }
                }
            }
            if world_version < 232 && bits.has_flags(8388608)? {
                self.world_z_rotation = b.get_int()? as f32;
            }
            if bits.has_flags(16777216)? {
                self.world_scale = b.get_float()?;
            }
            self.is_initialised = bits.has_flags(33554432)?;
            if bits.has_flags(67108864)? {
                self.components = Some(load_entity(b)?);
            }
            if bits.has_flags(134217728)? {
                return Err(not_ported("AnimalTracks.load"));
            }
            if bits.has_flags(268435456)? {
                self.texture = Some(read_string(b)?);
            }
            if bits.has_flags(536870912)? {
                self.model_index = b.get_int()?;
            }
            if bits.has_flags(1073741824)? {
                if world_version >= 232 {
                    self.world_x_rotation = b.get_float()?;
                    self.world_y_rotation = b.get_float()?;
                    self.world_z_rotation = b.get_float()?;
                } else {
                    self.world_y_rotation = b.get_int()? as f32;
                    self.world_x_rotation = b.get_int()? as f32;
                }
            }
        }
        Ok(())
    }
}
