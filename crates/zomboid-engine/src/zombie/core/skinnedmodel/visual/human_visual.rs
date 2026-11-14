//! zombie.core.skinnedmodel.visual.HumanVisual: skin, hair, beard, blood and the body's own item visuals.
use super::item_visual::{load_color, ItemVisual, BLOOD_BODY_PART_COUNT};
use crate::java_nio::{read_string, ByteBuffer};
use crate::zombie::util::string_utils;

/// zombie.core.skinnedmodel.visual.HumanVisual, the saved fields. Colours are the three unsigned bytes of
/// `new ImmutableColor(r, g, b)`; `skin_color` `None` is `ImmutableColor.white`.
#[derive(Clone, Debug, PartialEq)]
pub struct HumanVisual {
    pub skin_color: Option<[i32; 3]>,
    pub skin_texture: i32,
    pub skin_texture_name: Option<String>,
    pub zombie_rot_stage: i32,
    pub hair_color: Option<[i32; 3]>,
    pub beard_color: Option<[i32; 3]>,
    pub natural_hair_color: Option<[i32; 3]>,
    pub natural_beard_color: Option<[i32; 3]>,
    pub hair_model: Option<String>,
    pub beard_model: Option<String>,
    pub body_hair: i32,
    pub blood: [i8; BLOOD_BODY_PART_COUNT],
    pub dirt: [i8; BLOOD_BODY_PART_COUNT],
    pub holes: [i8; BLOOD_BODY_PART_COUNT],
    pub body_visuals: Vec<ItemVisual>,
    pub non_attached_hair: Option<String>,
}

impl Default for HumanVisual {
    /// zombie.core.skinnedmodel.visual.HumanVisual.clear
    fn default() -> Self {
        HumanVisual {
            skin_color: None,
            skin_texture: -1,
            skin_texture_name: None,
            zombie_rot_stage: -1,
            hair_color: None,
            beard_color: None,
            natural_hair_color: None,
            natural_beard_color: None,
            hair_model: None,
            beard_model: None,
            body_hair: -1,
            blood: [0; BLOOD_BODY_PART_COUNT],
            dirt: [0; BLOOD_BODY_PART_COUNT],
            holes: [0; BLOOD_BODY_PART_COUNT],
            body_visuals: Vec::new(),
            non_attached_hair: None,
        }
    }
}

/// `int count = input.get(); for (...) { byte amount = input.get(); if (i < arr.length) arr[i] = amount; }`
fn load_fixed(b: &mut ByteBuffer, arr: &mut [i8; BLOOD_BODY_PART_COUNT]) -> Result<(), String> {
    let count = i32::from(b.get()?);
    for i in 0..count.max(0) as usize {
        let amount = b.get()?;
        if i < arr.len() {
            arr[i] = amount;
        }
    }
    Ok(())
}

impl HumanVisual {
    /// zombie.core.skinnedmodel.visual.HumanVisual.load (after `clear()`).
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let mut v = HumanVisual::default();
        let flags1 = i32::from(b.get()?) & 255;
        if flags1 & 4 != 0 {
            v.hair_color = Some(load_color(b)?);
        }
        if flags1 & 2 != 0 {
            v.beard_color = Some(load_color(b)?);
        }
        if flags1 & 8 != 0 {
            v.skin_color = Some(load_color(b)?);
        }
        v.body_hair = i32::from(b.get()?);
        v.skin_texture = i32::from(b.get()?);
        v.zombie_rot_stage = i32::from(b.get()?);
        if flags1 & 64 != 0 {
            v.skin_texture_name = Some(read_string(b)?);
        }
        if flags1 & 16 != 0 {
            v.beard_model = Some(read_string(b)?);
        }
        if flags1 & 32 != 0 {
            v.hair_model = Some(read_string(b)?);
        }
        load_fixed(b, &mut v.blood)?;
        load_fixed(b, &mut v.dirt)?;
        load_fixed(b, &mut v.holes)?;
        let n = b.get()?;
        for _ in 0..n.max(0) {
            v.body_visuals.push(ItemVisual::load(b)?);
        }
        // setNonAttachedHair: null or whitespace becomes null.
        let hair = read_string(b)?;
        v.non_attached_hair = if string_utils::is_null_or_whitespace(Some(&hair)) {
            None
        } else {
            Some(hair)
        };
        let flags2 = i32::from(b.get()?) & 255;
        if flags2 & 4 != 0 {
            v.natural_hair_color = Some(load_color(b)?);
        }
        if flags2 & 2 != 0 {
            v.natural_beard_color = Some(load_color(b)?);
        }
        Ok(v)
    }
}
