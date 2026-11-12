use std::collections::HashMap;

use super::iso_meta_chunk::IsoMetaChunk;
use super::iso_meta_grid::{BuildingRef, RoomRef};
use super::room_def::RoomDef;

/// zombie.iso.IsoMetaCell: per-cell meta data: rooms and buildings from the lotheader, and the 32x32 meta chunks.
/// Zones, triggers and runtime rooms are not ported yet.
#[derive(Debug)]
pub struct IsoMetaCell {
    wx: i32,
    wy: i32,
    /// Priority index of the map folder whose header created the cell (`info.mapFiles`).
    pub info_priority: usize,
    chunk_map: Vec<Option<IsoMetaChunk>>,
    /// zombie.iso.IsoMetaCell.roomList: a room's position here is the index part of its id.
    pub room_list: Vec<RoomRef>,
    pub rooms: HashMap<i64, RoomRef>,
    /// zombie.iso.IsoMetaCell.roomByMetaId (a duplicate meta id keeps the later room).
    pub room_by_meta_id: HashMap<i64, RoomRef>,
    pub buildings: Vec<BuildingRef>,
    pub building_by_meta_id: HashMap<i64, BuildingRef>,
}

impl IsoMetaCell {
    pub fn new(wx: i32, wy: i32, info_priority: usize) -> Self {
        IsoMetaCell {
            wx,
            wy,
            info_priority,
            chunk_map: vec![None; 1024],
            room_list: Vec::new(),
            rooms: HashMap::new(),
            room_by_meta_id: HashMap::new(),
            buildings: Vec::new(),
            building_by_meta_id: HashMap::new(),
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
    /// zombie.iso.IsoMetaCell.hasChunk
    pub fn has_chunk(&self, x: i32, y: i32) -> bool {
        (0..32).contains(&x)
            && (0..32).contains(&y)
            && self.chunk_map[(y * 32 + x) as usize].is_some()
    }
    /// The chunk if it exists; `getChunk` of the original also creates it (see `IsoMetaGrid::get_chunk_data`).
    pub fn chunk(&self, i: usize) -> Option<&IsoMetaChunk> {
        self.chunk_map[i].as_ref()
    }
    /// zombie.iso.IsoMetaCell.getChunk (int): created on first use with `intensity`, the value of
    /// `LotHeader.getZombieIntensityForChunk` (negative means none, stored as 0).
    pub fn get_chunk(&mut self, i: usize, intensity: impl FnOnce() -> i32) -> &mut IsoMetaChunk {
        self.chunk_map[i].get_or_insert_with(|| {
            let mut c = IsoMetaChunk::default();
            let v = intensity();
            c.set_zombie_intensity(if v >= 0 { v as u8 } else { 0 });
            c
        })
    }
    /// zombie.iso.IsoMetaCell.addRoom: registers the room in every chunk of THIS cell its bounding box touches.
    /// `cell_x`/`cell_y` are the cell origin in squares. Java's truncating division is kept.
    pub fn add_room(
        &mut self,
        r: RoomRef,
        room: &RoomDef,
        cell_x: i32,
        cell_y: i32,
        mut intensity: impl FnMut(i32, i32) -> i32,
    ) {
        let mut xmax = room.x2 / 8;
        if room.x2 % 8 == 0 {
            xmax -= 1;
        }
        let mut ymax = room.y2 / 8;
        if room.y2 % 8 == 0 {
            ymax -= 1;
        }
        for yy in room.y / 8..=ymax {
            for xx in room.x / 8..=xmax {
                if xx >= cell_x / 8
                    && xx < (cell_x + 256) / 8
                    && yy >= cell_y / 8
                    && yy < (cell_y + 256) / 8
                {
                    let i = (xx - cell_x / 8 + (yy - cell_y / 8) * 32) as usize;
                    self.get_chunk(i, || intensity((i % 32) as i32, (i / 32) as i32))
                        .add_room(r);
                }
            }
        }
    }
}
