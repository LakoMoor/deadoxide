//! zombie.inventory.types.HandWeapon: weapon stats, attached parts and firearm state as a save stores them.
use crate::java_nio::{read_string, BitHeaderRead, ByteBuffer, HeaderSize};
use crate::zombie::inventory::inventory_item::{InventoryItem, ItemFactory, ItemType};

/// zombie.inventory.types.HandWeapon, the fields its load resets and reads. `None` leaves a field as the item script
/// made it (`fireMode`, `cyclicRateMultiplier`, the sight ranges; `weaponSprite` is reset to the script's).
#[derive(Clone, Debug, PartialEq)]
pub struct HandWeaponSave {
    pub max_range: f32,
    pub min_range_ranged: f32,
    pub clip_size: i32,
    pub min_damage: f32,
    pub max_damage: f32,
    pub recoil_delay: i32,
    pub aiming_time: i32,
    pub reload_time: i32,
    pub hit_chance: i32,
    pub min_angle: f32,
    /// The attached weapon parts (`attachWeaponPart(null, part, false)`: no stat changes), as full types in save
    /// order; flag 1024 stores whole items, flag 33554432 registry ids only.
    pub parts: Vec<String>,
    pub fire_mode: Option<String>,
    pub cyclic_rate_multiplier: Option<f32>,
    pub explosion_timer: i32,
    pub max_angle: f32,
    /// `setBloodLevel`: clamped to [0, 1].
    pub blood_level: f32,
    /// The saved flags; `setContainsClip`/`setRoundChambered` also need the script's magazine and chamber.
    pub contains_clip: bool,
    pub round_chambered: bool,
    pub is_jammed: bool,
    pub weapon_sprite: Option<String>,
    pub min_sight_range: Option<f32>,
    pub max_sight_range: Option<f32>,
}

impl HandWeaponSave {
    /// zombie.inventory.types.HandWeapon.load after `super.load` (`clearAllWeaponParts` runs before it).
    pub fn load(
        b: &mut ByteBuffer,
        world_version: i32,
        factory: &dyn ItemFactory,
    ) -> Result<Self, String> {
        let mut w = HandWeaponSave {
            max_range: 1.0,
            min_range_ranged: 0.0,
            clip_size: 0,
            min_damage: 0.4,
            max_damage: 1.5,
            recoil_delay: 0,
            aiming_time: 0,
            reload_time: 0,
            hit_chance: 0,
            min_angle: 0.5,
            parts: Vec::new(),
            fire_mode: None,
            cyclic_rate_multiplier: None,
            explosion_timer: 0,
            max_angle: 1.0,
            blood_level: 0.0,
            contains_clip: false,
            round_chambered: false,
            is_jammed: false,
            weapon_sprite: None,
            min_sight_range: None,
            max_sight_range: None,
        };
        let bits = BitHeaderRead::alloc_read(HeaderSize::Integer, b)?;
        if bits.equals(0) {
            return Ok(w);
        }
        if bits.has_flags(1)? {
            w.max_range = b.get_float()?;
        }
        if bits.has_flags(2)? {
            w.min_range_ranged = b.get_float()?;
        }
        if bits.has_flags(4)? {
            w.clip_size = b.get_int()?;
        }
        if bits.has_flags(8)? {
            w.min_damage = b.get_float()?;
        }
        if bits.has_flags(16)? {
            w.max_damage = b.get_float()?;
        }
        if bits.has_flags(32)? {
            w.recoil_delay = b.get_int()?;
        }
        if bits.has_flags(64)? {
            w.aiming_time = b.get_int()?;
        }
        if bits.has_flags(128)? {
            w.reload_time = b.get_int()?;
        }
        if bits.has_flags(256)? {
            w.hit_chance = b.get_int()?;
        }
        if bits.has_flags(512)? {
            w.min_angle = b.get_float()?;
        }
        if bits.has_flags(1024)? {
            let count = b.get()?;
            for _ in 0..count.max(0) {
                // InventoryItemFactory.CreateItem(input.getShort()); a null item throws (NullPointerException).
                let registry_id = b.get_short()?;
                let template = factory.create_item(registry_id).ok_or_else(|| {
                    "java.lang.NullPointerException: weapon part item".to_string()
                })?;
                b.get()?;
                let mut part = InventoryItem::new(registry_id, -1, template);
                part.load(b, world_version, factory)?;
                if part.item_type == ItemType::WeaponPart {
                    w.parts.push(part.full_type);
                }
            }
        }
        if bits.has_flags(2048)? {
            w.fire_mode = Some(read_string(b)?);
        }
        if bits.has_flags(4096)? {
            w.cyclic_rate_multiplier = Some(b.get_float()?);
        }
        if bits.has_flags(65536)? {
            w.explosion_timer = b.get_int()?;
        }
        if bits.has_flags(131072)? {
            w.max_angle = b.get_float()?;
        }
        if bits.has_flags(262144)? {
            w.blood_level = b.get_float()?.clamp(0.0, 1.0);
        }
        w.contains_clip = bits.has_flags(524288)?;
        w.round_chambered = bits.has_flags(1048576)?;
        w.is_jammed = bits.has_flags(2097152)?;
        if bits.has_flags(4194304)? {
            w.weapon_sprite = Some(read_string(b)?);
        }
        if bits.has_flags(8388608)? {
            w.min_sight_range = Some(b.get_float()?);
        }
        if bits.has_flags(16777216)? {
            w.max_sight_range = Some(b.get_float()?);
        }
        if bits.has_flags(33554432)? {
            let count = b.get()?;
            for _ in 0..count.max(0) {
                let registry_id = b.get_short()?;
                if let Some(t) = factory.create_item(registry_id) {
                    if t.item_type == ItemType::WeaponPart {
                        w.parts.push(t.full_type);
                    }
                }
            }
        }
        Ok(w)
    }
}
