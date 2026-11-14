//! zombie.inventory.types.Clothing: what a save stores for a worn item on top of `InventoryItem.load`.
use std::collections::BTreeMap;

use crate::java_nio::{read_string, BitHeaderRead, ByteBuffer, HeaderSize};

/// zombie.inventory.types.Clothing.ClothingPatch
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClothingPatch {
    pub tailor_lvl: i32,
    pub fabric_type: i32,
    pub scratch_defense: i32,
    pub bite_defense: i32,
    pub has_hole: bool,
    pub condition_gain: i32,
}

impl ClothingPatch {
    /// zombie.inventory.types.Clothing.ClothingPatch.load
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        Ok(ClothingPatch {
            tailor_lvl: i32::from(b.get()?),
            fabric_type: i32::from(b.get()?),
            scratch_defense: i32::from(b.get()?),
            bite_defense: i32::from(b.get()?),
            has_hole: b.get()? != 0,
            condition_gain: i32::from(b.get_short()?),
        })
    }
}

/// zombie.inventory.types.Clothing, the fields its load reads. `None` leaves a field as the item script made it.
/// `bloodLevel` and `dirtyness` are recomputed from the visual afterwards (`synchWithVisual`), and
/// `lastWetnessUpdate` becomes the world age; neither is modelled.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClothingSave {
    pub sprite_name: Option<String>,
    pub dirtyness: Option<f32>,
    pub blood_level: Option<f32>,
    /// The field starts at 0 and only the save changes it.
    pub wetness: f32,
    pub last_wetness_update: Option<f32>,
    /// `HashMap<Integer, ClothingPatch>`: body part index to patch (a repeated index replaces).
    pub patches: Option<BTreeMap<i32, ClothingPatch>>,
}

impl ClothingSave {
    /// zombie.inventory.types.Clothing.load after `super.load`.
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let mut c = ClothingSave::default();
        let bits = BitHeaderRead::alloc_read(HeaderSize::Byte, b)?;
        if !bits.equals(0) {
            if bits.has_flags(1)? {
                c.sprite_name = Some(read_string(b)?);
            }
            if bits.has_flags(2)? {
                c.dirtyness = Some(b.get_float()?);
            }
            if bits.has_flags(4)? {
                c.blood_level = Some(b.get_float()?);
            }
            if bits.has_flags(8)? {
                c.wetness = b.get_float()?;
            }
            if bits.has_flags(16)? {
                c.last_wetness_update = Some(b.get_float()?);
            }
            if bits.has_flags(32)? {
                let patch_nbr = i32::from(b.get()?);
                for _ in 0..patch_nbr.max(0) {
                    let part_index = i32::from(b.get()?);
                    let patch = ClothingPatch::load(b)?;
                    c.patches
                        .get_or_insert_with(BTreeMap::new)
                        .insert(part_index, patch);
                }
            }
        }
        Ok(c)
    }
}
