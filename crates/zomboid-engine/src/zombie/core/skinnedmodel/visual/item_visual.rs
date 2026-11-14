//! zombie.core.skinnedmodel.visual.ItemVisual: how a worn item looks (tint, texture choice, blood, holes, patches).
use crate::java_nio::{read_string, ByteBuffer};

/// zombie.characterTextures.BloodBodyPartType.MAX.index(): the length of the per-body-part arrays.
pub const BLOOD_BODY_PART_COUNT: usize = 18;

/// zombie.core.skinnedmodel.visual.ItemVisual, the saved fields.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemVisual {
    pub full_type: String,
    pub alternate_model_name: String,
    pub clothing_item_name: String,
    /// `new ImmutableColor(r, g, b)` from three unsigned bytes.
    pub tint: Option<[i32; 3]>,
    pub base_texture: i32,
    pub texture_choice: i32,
    pub hue: f32,
    pub decal: Option<String>,
    pub blood: Option<[i8; BLOOD_BODY_PART_COUNT]>,
    pub dirt: Option<[i8; BLOOD_BODY_PART_COUNT]>,
    pub holes: Option<[i8; BLOOD_BODY_PART_COUNT]>,
    pub basic_patches: Option<[i8; BLOOD_BODY_PART_COUNT]>,
    pub denim_patches: Option<[i8; BLOOD_BODY_PART_COUNT]>,
    pub leather_patches: Option<[i8; BLOOD_BODY_PART_COUNT]>,
}

impl Default for ItemVisual {
    /// The field initialisers of zombie.core.skinnedmodel.visual.ItemVisual.
    fn default() -> Self {
        ItemVisual {
            full_type: String::new(),
            alternate_model_name: String::new(),
            clothing_item_name: String::new(),
            tint: None,
            base_texture: -1,
            texture_choice: -1,
            hue: f32::INFINITY,
            decal: None,
            blood: None,
            dirt: None,
            holes: None,
            basic_patches: None,
            denim_patches: None,
            leather_patches: None,
        }
    }
}

/// `int count = input.get(); if (count > 0 && arr == null) arr = new byte[MAX]; for (...) if (i < arr.length) arr[i] = b`
pub(crate) fn load_part_bytes(
    b: &mut ByteBuffer,
    arr: &mut Option<[i8; BLOOD_BODY_PART_COUNT]>,
) -> Result<(), String> {
    let count = i32::from(b.get()?);
    if count > 0 && arr.is_none() {
        *arr = Some([0; BLOOD_BODY_PART_COUNT]);
    }
    for i in 0..count.max(0) as usize {
        let amount = b.get()?;
        if let Some(a) = arr.as_mut() {
            if i < a.len() {
                a[i] = amount;
            }
        }
    }
    Ok(())
}

/// Three unsigned bytes, as `new ImmutableColor(r, g, b)` reads them.
pub(crate) fn load_color(b: &mut ByteBuffer) -> Result<[i32; 3], String> {
    Ok([
        i32::from(b.get()?) & 255,
        i32::from(b.get()?) & 255,
        i32::from(b.get()?) & 255,
    ])
}

impl ItemVisual {
    /// zombie.core.skinnedmodel.visual.ItemVisual.load
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let mut v = ItemVisual::default();
        let flags1 = i32::from(b.get()?) & 255;
        v.full_type = read_string(b)?;
        v.alternate_model_name = read_string(b)?;
        v.clothing_item_name = read_string(b)?;
        if flags1 & 1 != 0 {
            v.tint = Some(load_color(b)?);
        }
        if flags1 & 2 != 0 {
            v.base_texture = i32::from(b.get()?);
        }
        if flags1 & 4 != 0 {
            v.texture_choice = i32::from(b.get()?);
        }
        if flags1 & 8 != 0 {
            v.hue = b.get_float()?;
        }
        if flags1 & 16 != 0 {
            v.decal = Some(read_string(b)?);
        }
        load_part_bytes(b, &mut v.blood)?;
        load_part_bytes(b, &mut v.dirt)?;
        load_part_bytes(b, &mut v.holes)?;
        load_part_bytes(b, &mut v.basic_patches)?;
        load_part_bytes(b, &mut v.denim_patches)?;
        load_part_bytes(b, &mut v.leather_patches)?;
        Ok(v)
    }
}
