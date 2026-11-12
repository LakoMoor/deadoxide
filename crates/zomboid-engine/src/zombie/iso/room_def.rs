use super::iso_meta_grid::BuildingRef;
use super::meta_object::MetaObject;
use crate::java;

/// zombie.iso.RoomDef.RoomRect
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct RoomRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
impl RoomRect {
    /// zombie.iso.RoomDef.RoomRect.getX2
    pub fn get_x2(&self) -> i32 {
        self.x + self.w
    }
    /// zombie.iso.RoomDef.RoomRect.getY2
    pub fn get_y2(&self) -> i32 {
        self.y + self.h
    }
}

/// zombie.iso.RoomDef: a room as the map files define it. `building` is a handle into `IsoMetaGrid::building_defs`.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomDef {
    pub explored: bool,
    pub done_spawn: bool,
    pub indoor_zombies: i32,
    pub spawn_count: i32,
    pub lights_active: bool,
    pub name: String,
    pub level: i32,
    pub building: Option<BuildingRef>,
    pub id: i64,
    pub rects: Vec<RoomRect>,
    pub objects: Vec<MetaObject>,
    pub x: i32,
    pub y: i32,
    pub x2: i32,
    pub y2: i32,
    pub area: i32,
    pub meta_id: i64,
    pub user_defined: bool,
}

impl RoomDef {
    pub fn new(id: i64, name: String) -> Self {
        RoomDef {
            explored: false,
            done_spawn: false,
            indoor_zombies: 0,
            spawn_count: -1,
            lights_active: false,
            name,
            level: 0,
            building: None,
            id,
            rects: Vec::new(),
            objects: Vec::new(),
            x: 100000,
            y: 100000,
            x2: -10000,
            y2: -10000,
            area: 0,
            meta_id: 0,
            user_defined: false,
        }
    }

    /// zombie.iso.RoomDef.CalculateBounds
    pub fn calculate_bounds(&mut self) {
        self.x = 10000000;
        self.y = 10000000;
        self.x2 = -1000000;
        self.y2 = -1000000;
        self.area = 0;
        for rect in &self.rects {
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
            self.area = self.area.wrapping_add(rect.w.wrapping_mul(rect.h));
        }
    }

    /// zombie.iso.RoomDef.calculateMetaID: level and the cell-relative position of one rect, stable across map versions.
    /// The rect picked is the original's (`x <= minX && y < minY`), not a true minimum.
    pub fn calculate_meta_id(&self, cell_x: i32, cell_y: i32) -> i64 {
        let (mut min_x, mut min_y) = (i32::MAX, i32::MAX);
        for rect in &self.rects {
            if rect.x <= min_x && rect.y < min_y {
                min_x = rect.x;
                min_y = rect.y;
            }
        }
        min_x = min_x.wrapping_sub(cell_x.wrapping_mul(256));
        min_y = min_y.wrapping_sub(cell_y.wrapping_mul(256));
        (self.level as i64) << 32 | (min_y as i64) << 16 | min_x as i64
    }

    /// zombie.iso.RoomDef.isEmptyOutside
    pub fn is_empty_outside(&self) -> bool {
        java::equals_ignore_case("emptyoutside", &self.name)
    }

    /// zombie.iso.RoomDef.getRoomRect: bounds are inclusive on both ends, as written in the original.
    pub fn get_room_rect(&self, x: i32, y: i32) -> Option<&RoomRect> {
        self.rects
            .iter()
            .find(|r| r.x <= x && r.y <= y && r.get_x2() >= x && r.get_y2() >= y)
    }

    /// zombie.iso.RoomDef.RoomRect.contains through zombie.iso.RoomDef.contains: the ints are compared as floats there.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        let (x, y) = (x as f32, y as f32);
        self.rects.iter().any(|r| {
            x >= r.x as f32 && y >= r.y as f32 && x < r.get_x2() as f32 && y < r.get_y2() as f32
        })
    }

    /// zombie.iso.RoomDef.intersects
    pub fn intersects(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        self.rects
            .iter()
            .any(|r| x + w > r.x && x < r.get_x2() && y + h > r.y && y < r.get_y2())
    }

    /// zombie.iso.RoomDef.getArea
    pub fn get_area(&self) -> i32 {
        self.area
    }
}
