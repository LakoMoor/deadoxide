use super::iso_meta_grid::RoomRef;
use super::room_def::RoomDef;
use crate::zombie::core::math::pz_math;

/// zombie.iso.BuildingDef: the map-file side of a building. `rooms` and `emptyoutside` are handles into
/// `IsoMetaGrid::room_defs` (the original holds RoomDef references).
#[derive(Clone, Debug, PartialEq)]
pub struct BuildingDef {
    pub emptyoutside: Vec<RoomRef>,
    pub rooms: Vec<RoomRef>,
    /// Pairs (chunk x, chunk y) as shorts, in insertion order.
    pub overlapped_chunks: Vec<i16>,
    pub alarmed: bool,
    pub alarm_decay: i32,
    pub x: i32,
    pub y: i32,
    pub x2: i32,
    pub y2: i32,
    pub id: i64,
    pub meta_id: i64,
}

impl Default for BuildingDef {
    fn default() -> Self {
        BuildingDef {
            emptyoutside: Vec::new(),
            rooms: Vec::new(),
            overlapped_chunks: Vec::new(),
            alarmed: false,
            alarm_decay: 10000000,
            x: 10000000,
            y: 10000000,
            x2: -10000000,
            y2: -10000000,
            id: 0,
            meta_id: 0,
        }
    }
}

impl BuildingDef {
    /// zombie.iso.BuildingDef.CalculateBounds: bounding box over all rects, then the list of chunks any rect touches
    /// (with a one-square margin on the low side, integer division truncating toward zero as in Java).
    pub fn calculate_bounds(&mut self, room_list: &[RoomDef]) {
        self.x = i32::MAX;
        self.y = i32::MAX;
        self.x2 = i32::MIN;
        self.y2 = i32::MIN;
        for &r in self.rooms.iter().chain(&self.emptyoutside) {
            for rect in &room_list[r.0 as usize].rects {
                if rect.x < self.x {
                    self.x = rect.x;
                }
                if rect.y < self.y {
                    self.y = rect.y;
                }
                if rect.x.wrapping_add(rect.w) > self.x2 {
                    self.x2 = rect.x.wrapping_add(rect.w);
                }
                if rect.y.wrapping_add(rect.h) > self.y2 {
                    self.y2 = rect.y.wrapping_add(rect.h);
                }
            }
        }
        self.overlapped_chunks.clear();
        let all: Vec<RoomRef> = self
            .rooms
            .iter()
            .chain(&self.emptyoutside)
            .copied()
            .collect();
        for r in all {
            for rect in &room_list[r.0 as usize].rects {
                let min_x = rect.x.wrapping_sub(1) / 8;
                let min_y = rect.y.wrapping_sub(1) / 8;
                let max_x = rect.x.wrapping_add(rect.w) / 8;
                let max_y = rect.y.wrapping_add(rect.h) / 8;
                for wy in min_y..=max_y {
                    for wx in min_x..=max_x {
                        if !self.overlaps_chunk(wx, wy) {
                            self.overlapped_chunks.push(wx as i16);
                            self.overlapped_chunks.push(wy as i16);
                        }
                    }
                }
            }
        }
    }

    /// zombie.iso.BuildingDef.calculateMetaID: lowest level, then the same rect choice as `RoomDef.calculateMetaID`.
    pub fn calculate_meta_id(&self, room_list: &[RoomDef], cell_x: i32, cell_y: i32) -> i64 {
        let (mut min_x, mut min_y, mut min_z) = (i32::MAX, i32::MAX, i32::MAX);
        let room_defs = if self.rooms.is_empty() {
            &self.emptyoutside
        } else {
            &self.rooms
        };
        for &r in room_defs {
            let room_def = &room_list[r.0 as usize];
            if room_def.level <= min_z {
                if room_def.level < min_z {
                    min_x = i32::MAX;
                    min_y = i32::MAX;
                }
                min_z = room_def.level;
                for rect in &room_def.rects {
                    if rect.x <= min_x && rect.y < min_y {
                        min_x = rect.x;
                        min_y = rect.y;
                    }
                }
            }
        }
        min_x = min_x.wrapping_sub(cell_x.wrapping_mul(256));
        min_y = min_y.wrapping_sub(cell_y.wrapping_mul(256));
        (min_z as i64) << 32 | (min_y as i64) << 16 | min_x as i64
    }

    /// zombie.iso.BuildingDef.overlapsChunk
    pub fn overlaps_chunk(&self, wx: i32, wy: i32) -> bool {
        self.overlapped_chunks
            .chunks_exact(2)
            .any(|c| wx == c[0] as i32 && wy == c[1] as i32)
    }

    /// Chunk range of the bounding box, as `CalculateBounds` computes it before listing overlapped chunks.
    pub fn bounds_chunks(&self) -> (i32, i32, i32, i32) {
        (
            pz_math::fastfloor(self.x as f32 / 8.0),
            pz_math::fastfloor(self.y as f32 / 8.0),
            pz_math::fastfloor(self.x2 as f32 / 8.0),
            pz_math::fastfloor(self.y2 as f32 / 8.0),
        )
    }
}
