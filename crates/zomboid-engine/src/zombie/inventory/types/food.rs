//! zombie.inventory.types.Food: age, nutrition, cooking and freezing state as a save stores them.
use crate::java_nio::{read_string, BitHeaderRead, ByteBuffer, HeaderSize};

/// zombie.inventory.types.Food, the fields its load resets to constants and then reads (so all of them come from the
/// save). The milk fields are not reset; egg data (flag 134217728) is not ported.
#[derive(Clone, Debug, PartialEq)]
pub struct FoodSave {
    pub age: f32,
    /// As saved; on a server `-1` becomes the world age after loading.
    pub last_aged: f32,
    pub calories: f32,
    pub proteins: f32,
    pub lipids: f32,
    pub carbohydrates: f32,
    pub hung_change: f32,
    pub base_hunger: f32,
    pub unhappy_change: f32,
    pub boredom_change: f32,
    pub thirst_change: f32,
    pub heat: f32,
    pub last_cook_minute: i32,
    pub cooking_time: f32,
    pub cooked: bool,
    pub burnt: bool,
    pub is_cookable: bool,
    pub dangerous_uncooked: bool,
    pub poison_detection_level: i32,
    pub spices: Option<Vec<String>>,
    pub poison_power: i32,
    pub chef: Option<String>,
    pub off_age: i32,
    pub off_age_max: i32,
    pub pain_reduction: f32,
    pub flu_reduction: i32,
    pub food_sickness_change: i32,
    pub poison: bool,
    pub use_for_poison: i32,
    pub freezing_time: f32,
    pub frozen: bool,
    pub last_frozen_update: f32,
    pub rotten_time: f32,
    pub compost_time: f32,
    pub cooked_in_microwave: bool,
    pub fatigue_change: f32,
    pub end_change: f32,
    /// (milkQty, milkType) where saved.
    pub milk: Option<(i32, String)>,
    pub stress_change: f32,
}

impl FoodSave {
    /// zombie.inventory.types.Food.load after `super.load`.
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let mut f = FoodSave {
            age: 0.0,
            last_aged: 0.0,
            calories: 0.0,
            proteins: 0.0,
            lipids: 0.0,
            carbohydrates: 0.0,
            hung_change: 0.0,
            base_hunger: 0.0,
            unhappy_change: 0.0,
            boredom_change: 0.0,
            thirst_change: 0.0,
            heat: 1.0,
            last_cook_minute: 0,
            cooking_time: 0.0,
            cooked: false,
            burnt: false,
            is_cookable: false,
            dangerous_uncooked: false,
            poison_detection_level: -1,
            spices: None,
            poison_power: 0,
            chef: None,
            off_age: 1_000_000_000,
            off_age_max: 1_000_000_000,
            pain_reduction: 0.0,
            flu_reduction: 0,
            food_sickness_change: 0,
            poison: false,
            use_for_poison: 0,
            freezing_time: 0.0,
            frozen: false,
            last_frozen_update: 0.0,
            rotten_time: 0.0,
            compost_time: 0.0,
            cooked_in_microwave: false,
            fatigue_change: 0.0,
            end_change: 0.0,
            milk: None,
            stress_change: 0.0,
        };
        f.age = b.get_float()?;
        f.last_aged = b.get_float()?;
        let header = BitHeaderRead::alloc_read(HeaderSize::Byte, b)?;
        if !header.equals(0) {
            if header.has_flags(1)? {
                f.calories = b.get_float()?;
                f.proteins = b.get_float()?;
                f.lipids = b.get_float()?;
                f.carbohydrates = b.get_float()?;
            }
            if header.has_flags(2)? {
                f.hung_change = b.get_float()?;
            }
            if header.has_flags(4)? {
                f.base_hunger = b.get_float()?;
            }
            if header.has_flags(8)? {
                f.unhappy_change = b.get_float()?;
            }
            if header.has_flags(16)? {
                f.boredom_change = b.get_float()?;
            }
            if header.has_flags(32)? {
                f.thirst_change = b.get_float()?;
            }
            if header.has_flags(64)? {
                let bits = BitHeaderRead::alloc_read(HeaderSize::Integer, b)?;
                if bits.has_flags(1)? {
                    f.heat = b.get_float()?;
                }
                if bits.has_flags(2)? {
                    f.last_cook_minute = b.get_int()?;
                }
                if bits.has_flags(4)? {
                    f.cooking_time = b.get_float()?;
                }
                f.cooked = bits.has_flags(8)?;
                f.burnt = bits.has_flags(16)?;
                f.is_cookable = bits.has_flags(32)?;
                f.dangerous_uncooked = bits.has_flags(64)?;
                if bits.has_flags(128)? {
                    f.poison_detection_level = i32::from(b.get()?);
                }
                if bits.has_flags(256)? {
                    let size = i32::from(b.get()?);
                    let mut spices = Vec::new();
                    for _ in 0..size.max(0) {
                        spices.push(read_string(b)?);
                    }
                    f.spices = Some(spices);
                }
                if bits.has_flags(512)? {
                    f.poison_power = i32::from(b.get()?);
                }
                if bits.has_flags(1024)? {
                    f.chef = Some(read_string(b)?);
                }
                if bits.has_flags(2048)? {
                    f.off_age = b.get_int()?;
                }
                if bits.has_flags(4096)? {
                    f.off_age_max = b.get_int()?;
                }
                if bits.has_flags(8192)? {
                    f.pain_reduction = b.get_float()?;
                }
                if bits.has_flags(16384)? {
                    f.flu_reduction = b.get_int()?;
                }
                if bits.has_flags(32768)? {
                    f.food_sickness_change = b.get_int()?;
                }
                f.poison = bits.has_flags(65536)?;
                if bits.has_flags(131072)? {
                    f.use_for_poison = i32::from(b.get_short()?);
                }
                if bits.has_flags(262144)? {
                    f.freezing_time = b.get_float()?;
                }
                f.frozen = bits.has_flags(524288)?;
                if bits.has_flags(1048576)? {
                    f.last_frozen_update = b.get_float()?;
                }
                if bits.has_flags(2097152)? {
                    f.rotten_time = b.get_float()?;
                }
                if bits.has_flags(4194304)? {
                    f.compost_time = b.get_float()?;
                }
                f.cooked_in_microwave = bits.has_flags(8388608)?;
                if bits.has_flags(16777216)? {
                    f.fatigue_change = b.get_float()?;
                }
                if bits.has_flags(33554432)? {
                    f.end_change = b.get_float()?;
                }
                if bits.has_flags(67108864)? {
                    let qty = b.get_int()?;
                    f.milk = Some((qty, read_string(b)?));
                }
                if bits.has_flags(134217728)? {
                    return Err(super::super::inventory_item::not_ported(
                        "Food.load: fertilized egg data",
                    ));
                }
                if bits.has_flags(268435456)? {
                    f.stress_change = b.get_float()?;
                }
            }
        }
        Ok(f)
    }
}
