use super::iso_meta_grid::RoomRef;
use super::room_def::RoomDef;

/// zombie.iso.IsoMetaChunk: per-chunk meta data: zombie intensity and the rooms overlapping the chunk. Zones are not ported yet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IsoMetaChunk {
    zombie_intensity: u8,
    rooms: Vec<RoomRef>,
}

impl IsoMetaChunk {
    /// zombie.iso.IsoMetaChunk.getRoomsSize
    pub fn get_rooms_size(&self) -> usize {
        self.rooms.len()
    }
    pub fn rooms(&self) -> &[RoomRef] {
        &self.rooms
    }
    /// zombie.iso.IsoMetaChunk.setZombieIntensity
    pub fn set_zombie_intensity(&mut self, v: u8) {
        self.zombie_intensity = v;
    }
    /// zombie.iso.IsoMetaChunk.getUnadjustedZombieIntensity
    pub fn get_unadjusted_zombie_intensity(&self) -> i32 {
        self.zombie_intensity as i32
    }
    /// zombie.iso.IsoMetaChunk.addRoom
    pub fn add_room(&mut self, room: RoomRef) {
        self.rooms.push(room);
    }
    /// zombie.iso.IsoMetaChunk.removeRoom
    pub fn remove_room(&mut self, room: RoomRef) {
        if let Some(i) = self.rooms.iter().position(|r| *r == room) {
            self.rooms.remove(i);
        }
    }
    /// zombie.iso.IsoMetaChunk.getRoomAt: the room added last wins, a user-defined one before any other.
    pub fn get_room_at(&self, defs: &[RoomDef], x: i32, y: i32, z: i32) -> Option<RoomRef> {
        let mut non_user_defined = None;
        for &r in self.rooms.iter().rev() {
            let room = &defs[r.0 as usize];
            if !room.is_empty_outside() && room.level == z && room.contains(x, y) {
                if room.user_defined {
                    return Some(r);
                }
                if non_user_defined.is_none() {
                    non_user_defined = Some(r);
                }
            }
        }
        non_user_defined
    }
    /// zombie.iso.IsoMetaChunk.getEmptyOutsideAt
    pub fn get_empty_outside_at(
        &self,
        defs: &[RoomDef],
        x: i32,
        y: i32,
        z: i32,
    ) -> Option<RoomRef> {
        self.rooms.iter().copied().find(|r| {
            let room = &defs[r.0 as usize];
            room.is_empty_outside() && room.level == z && room.contains(x, y)
        })
    }
}
