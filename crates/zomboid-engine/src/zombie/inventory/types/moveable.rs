//! zombie.inventory.types.Moveable: a picked-up furniture item (its world sprite and lamp state).
use crate::java_nio::{read_string, ByteBuffer};

/// The lamp fields of zombie.inventory.types.Moveable, saved where `isLight`.
#[derive(Clone, Debug, PartialEq)]
pub struct MoveableLight {
    pub use_battery: bool,
    pub has_battery: bool,
    pub bulb_item: Option<String>,
    pub power: f32,
    pub delta: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// zombie.inventory.types.Moveable, the saved fields. `ReadFromWorldSprite` (which takes the weight, name and type
/// from the sprite's properties) is not ported.
#[derive(Clone, Debug, PartialEq)]
pub struct MoveableSave {
    pub world_sprite: String,
    /// `Some` where `isLight` was saved true.
    pub light: Option<MoveableLight>,
}

impl MoveableSave {
    /// zombie.inventory.types.Moveable.load after `super.load`.
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let world_sprite = read_string(b)?;
        let is_light = b.get()? != 0;
        let light = if is_light {
            let use_battery = b.get()? != 0;
            let has_battery = b.get()? != 0;
            let bulb_item = if b.get()? != 0 {
                Some(read_string(b)?)
            } else {
                None
            };
            Some(MoveableLight {
                use_battery,
                has_battery,
                bulb_item,
                power: b.get_float()?,
                delta: b.get_float()?,
                r: b.get_float()?,
                g: b.get_float()?,
                b: b.get_float()?,
            })
        } else {
            None
        };
        Ok(MoveableSave {
            world_sprite,
            light,
        })
    }
}
