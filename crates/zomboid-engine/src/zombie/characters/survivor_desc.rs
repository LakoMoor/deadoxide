//! zombie.characters.SurvivorDesc: a character's name, gender, profession, extras, XP boosts and voice.
use crate::java_nio::{read_string, ByteBuffer};

/// zombie.characters.SurvivorDesc, the saved fields. The profession and perk names are kept as saved (the original
/// resolves them through its registries; an unknown perk is dropped there).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurvivorDesc {
    pub id: i32,
    pub forename: String,
    pub surname: String,
    pub torso: String,
    pub female: bool,
    pub profession: String,
    pub extra: Vec<String>,
    /// (perk name, level) in save order
    pub xp_boosts: Vec<(String, i32)>,
    pub voice_prefix: String,
    pub voice_pitch: f32,
    pub voice_type: i32,
}

impl SurvivorDesc {
    /// zombie.characters.SurvivorDesc.load. World versions before 208 (which draw a random voice) are refused.
    pub fn load(b: &mut ByteBuffer, world_version: i32) -> Result<Self, String> {
        let mut d = SurvivorDesc {
            id: b.get_int()?,
            ..SurvivorDesc::default()
        };
        d.forename = read_string(b)?;
        d.surname = read_string(b)?;
        d.torso = read_string(b)?;
        d.female = b.get_int()? == 1;
        d.profession = read_string(b)?;
        if b.get_int()? == 1 {
            let size = b.get_int()?;
            for _ in 0..size {
                d.extra.push(read_string(b)?);
            }
        }
        let size = b.get_int()?;
        for _ in 0..size {
            // loadPerk
            let perk = read_string(b)?;
            let level = b.get_int()?;
            d.xp_boosts.push((perk, level));
        }
        if world_version < 208 {
            return Err("survivor descriptors of world versions before 208 are not ported".into());
        }
        d.voice_prefix = read_string(b)?;
        d.voice_pitch = b.get_float()?;
        d.voice_type = b.get_int()?;
        Ok(d)
    }
}
