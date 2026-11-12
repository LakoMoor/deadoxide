use std::collections::HashMap;

use super::building_def::BuildingDef;
use super::room_def::RoomDef;

/// zombie.iso.IsoMetaCell: per-cell meta data. So far the rooms and buildings read from the lotheader;
/// zones, chunks, triggers and runtime rooms are not ported yet.
#[derive(Debug, Default)]
pub struct IsoMetaCell {
    wx: i32,
    wy: i32,
    /// zombie.iso.IsoMetaCell.roomList: owner of the RoomDefs; a room's index here is the index part of its id.
    pub room_list: Vec<RoomDef>,
    /// zombie.iso.IsoMetaCell.rooms: id -> index in `room_list`.
    pub rooms: HashMap<i64, usize>,
    /// zombie.iso.IsoMetaCell.roomByMetaId: meta id -> index in `room_list` (a duplicate keeps the later room).
    pub room_by_meta_id: HashMap<i64, usize>,
    pub buildings: Vec<BuildingDef>,
    /// zombie.iso.IsoMetaCell.buildingByMetaId: meta id -> index in `buildings`.
    pub building_by_meta_id: HashMap<i64, usize>,
}

impl IsoMetaCell {
    pub fn new(wx: i32, wy: i32) -> Self {
        IsoMetaCell {
            wx,
            wy,
            ..Default::default()
        }
    }
    /// zombie.iso.IsoMetaCell.getX
    pub fn get_x(&self) -> i32 {
        self.wx
    }
    /// zombie.iso.IsoMetaCell.getY
    pub fn get_y(&self) -> i32 {
        self.wy
    }
}
