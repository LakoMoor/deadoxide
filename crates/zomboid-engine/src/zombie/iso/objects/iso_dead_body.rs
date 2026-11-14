//! zombie.iso.objects.IsoDeadBody: a corpse lying on a square (`IsoGridSquare.staticMovingObjects`), as a save
//! stores it. Animal corpses are not ported.
use crate::java_nio::{read_string, ByteBuffer};
use crate::se::krka::kahlua::j2se::kahlua_table_impl::KahluaTable;
use crate::zombie::characters::survivor_desc::SurvivorDesc;
use crate::zombie::core::skinnedmodel::visual::human_visual::HumanVisual;
use crate::zombie::inventory::inventory_item::ItemFactory;
use crate::zombie::inventory::item_container::ItemContainer;

/// zombie.iso.objects.IsoDeadBody, the saved fields.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoDeadBody {
    // zombie.iso.IsoMovingObject.load
    pub offset_x: f32,
    pub offset_y: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub dir: i32,
    pub mod_data: Option<KahluaTable>,
    // zombie.iso.objects.IsoDeadBody.load
    pub female: bool,
    pub was_zombie: bool,
    /// zombie.network.id.ObjectID.ObjectIDShort: (id, type index)
    pub object_id: (i16, i8),
    pub server: bool,
    pub persistent_outfit_id: i32,
    pub desc: Option<SurvivorDesc>,
    pub human_visual: HumanVisual,
    pub container: Option<ItemContainer>,
    /// (body location, index into the saved items)
    pub worn_items: Vec<(String, i16)>,
    /// (attachment location, index into the saved items)
    pub attached_items: Vec<(String, i16)>,
    pub death_time: f32,
    pub reanimate_time: f32,
    pub fall_on_front: bool,
    pub killed_by_fall: bool,
    pub was_skeleton: bool,
    pub angle: f32,
    pub zombie_rot_stage_at_death: i32,
    pub animal_rot_stage_at_death: i32,
    pub rotten_texture: Option<String>,
    pub skel_inv_icon: Option<String>,
    pub crawling: bool,
    pub fake_dead: bool,
    pub ragdoll_fall: bool,
    /// (bone id, position xyz, rotation xyzw, scale xyz) as saved.
    pub died_bone_transforms: Vec<(i32, [f32; 10])>,
}

impl IsoDeadBody {
    /// zombie.iso.objects.IsoDeadBody.load (with zombie.iso.IsoMovingObject.load first). The worn and attached item
    /// lists are kept as saved; the original drops entries whose location its body-location group does not know.
    pub fn load(
        b: &mut ByteBuffer,
        world_version: i32,
        items: &dyn ItemFactory,
    ) -> Result<Self, String> {
        let offset_x = b.get_float()?;
        let offset_y = b.get_float()?;
        // IsoWorld.saveoffsetx/saveoffsety are 0 outside the old world-shift code.
        let x = b.get_float()?;
        let y = b.get_float()?;
        let z = b.get_float()?;
        let dir = b.get_int()?;
        let mod_data = if b.get()? != 0 {
            Some(KahluaTable::load(b, world_version)?)
        } else {
            None
        };
        let female = b.get()? != 0;
        let was_zombie = b.get()? != 0;
        if b.get()? != 0 {
            return Err("loading animal corpses is not ported".into());
        }
        if world_version < 199 {
            return Err("corpses of world versions before 199 are not ported".into());
        }
        let object_id = (b.get_short()?, b.get()?);
        let server = b.get()? != 0;
        let persistent_outfit_id = b.get_int()?;
        let desc = if b.get()? != 0 {
            Some(SurvivorDesc::load(b, world_version)?)
        } else {
            None
        };
        let visual_type = b.get()?;
        let human_visual = match visual_type {
            0 => HumanVisual::load(b)?,
            1 => return Err("animal visuals of corpses are not ported".into()),
            _ => return Err("java.io.IOException: invalid visualType for corpse".into()),
        };
        let mut container = None;
        let mut worn_items = Vec::new();
        let mut attached_items = Vec::new();
        if b.get()? != 0 {
            let mut c = ItemContainer {
                id: b.get_int()?,
                capacity: 50,
                ..ItemContainer::default()
            };
            c.load(b, world_version, items)?;
            let n = b.get()?;
            for _ in 0..n.max(0) {
                let location = read_string(b)?;
                worn_items.push((location, b.get_short()?));
            }
            let n = b.get()?;
            for _ in 0..n.max(0) {
                let location = read_string(b)?;
                attached_items.push((location, b.get_short()?));
            }
            container = Some(c);
        }
        let death_time = b.get_float()?;
        let reanimate_time = b.get_float()?;
        let flags = i32::from(b.get()?);
        let was_skeleton = b.get()? != 0;
        let angle = b.get_float()?;
        let zombie_rot_stage_at_death = i32::from(b.get()?) & 255;
        let animal_rot_stage_at_death = if world_version >= 222 {
            i32::from(b.get()?) & 255
        } else {
            0
        };
        let (rotten_texture, skel_inv_icon) = if world_version >= 225 {
            (Some(read_string(b)?), Some(read_string(b)?))
        } else {
            (None, None)
        };
        let crawling = b.get()? != 0;
        let fake_dead = b.get()? != 0;
        let ragdoll_fall = b.get()? != 0;
        let mut died_bone_transforms = Vec::new();
        if ragdoll_fall {
            let bone_count = b.get_int()?;
            for _ in 0..bone_count.max(0) {
                let bone_id = b.get_int()?;
                let mut t = [0.0f32; 10];
                for v in &mut t {
                    *v = b.get_float()?;
                }
                if bone_id >= 0 && bone_id < bone_count {
                    died_bone_transforms.push((bone_id, t));
                }
            }
        }
        Ok(IsoDeadBody {
            offset_x,
            offset_y,
            x,
            y,
            z,
            dir,
            mod_data,
            female,
            was_zombie,
            object_id,
            server,
            persistent_outfit_id,
            desc,
            human_visual,
            container,
            worn_items,
            attached_items,
            death_time,
            reanimate_time,
            fall_on_front: flags & 1 != 0,
            killed_by_fall: flags & 2 != 0,
            was_skeleton,
            angle,
            zombie_rot_stage_at_death,
            animal_rot_stage_at_death,
            rotten_texture,
            skel_inv_icon,
            crawling,
            fake_dead,
            ragdoll_fall,
            died_bone_transforms,
        })
    }
}
