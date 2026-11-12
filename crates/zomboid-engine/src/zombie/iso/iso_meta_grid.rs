//! zombie.iso.IsoMetaGrid: the world's meta data. So far: map folders, cell headers, rooms and buildings.
use std::path::Path;

use super::building_def::BuildingDef;
use super::building_id;
use super::iso_directions::IsoDirections;
use super::iso_lot::{self, RandomAccess};
use super::iso_meta_cell::IsoMetaCell;
use super::lot_header::{LotHeader, LOTHEADER_MAGIC};
use super::map_files::MapFiles;
use super::meta_object::MetaObject;
use super::room_def::{RoomDef, RoomRect};
use super::room_id;
use crate::java;
use crate::zombie::core::math::pz_math;
use crate::zombie::game_states::choose_game_info;

/// Handle to a `RoomDef` in `IsoMetaGrid::room_defs` (the original passes object references).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RoomRef(pub u32);
/// Handle to a `BuildingDef` in `IsoMetaGrid::building_defs`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BuildingRef(pub u32);

/// zombie.iso.areas.IsoRoom: the runtime room made from a `RoomDef` the first time a square of it is loaded.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoRoom {
    pub def: RoomRef,
    /// Index into `IsoMetaGrid::iso_buildings`.
    pub building: usize,
}

/// zombie.iso.areas.IsoBuilding: the runtime building; `rooms` are indices into `IsoMetaGrid::iso_rooms`.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoBuilding {
    pub id: i32,
    pub score_update: i32,
    pub def: BuildingRef,
    pub rooms: Vec<usize>,
}

/// The sandbox values `CreateStep2` reads when it decides which buildings are alarmed.
#[derive(Clone, Copy, Debug)]
pub struct AlarmSettings {
    /// `Core.gameMode.equals("LastStand")`
    pub last_stand: bool,
    /// `SandboxOptions.instance.doesPowerGridExist()`
    pub power_grid_exists: bool,
    /// `SandboxOptions.instance.alarm.getValue()`
    pub alarm: i32,
    /// `SandboxOptions.getInstance().alarmDecay.getValue()`
    pub alarm_decay: i32,
}

/// zombie.iso.IsoMetaGrid. Also holds what the original keeps in statics of `IsoLot` (`MapFiles`).
#[derive(Debug, Default)]
pub struct IsoMetaGrid {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub min_non_procedural_x: i32,
    pub min_non_procedural_y: i32,
    pub max_non_procedural_x: i32,
    pub max_non_procedural_y: i32,
    width: i32,
    height: i32,
    grid: Vec<Option<Box<IsoMetaCell>>>,
    /// zombie.iso.IsoLot.MapFiles, in priority order.
    pub map_files: Vec<MapFiles>,
    pub room_defs: Vec<RoomDef>,
    pub building_defs: Vec<BuildingDef>,
    /// zombie.iso.IsoMetaGrid.buildings
    pub buildings: Vec<BuildingRef>,
    pub iso_rooms: Vec<IsoRoom>,
    pub iso_buildings: Vec<IsoBuilding>,
    /// zombie.iso.IsoMetaCell.isoRooms of all cells (room ids carry their cell).
    pub iso_room_by_id: std::collections::HashMap<i64, usize>,
    /// zombie.iso.IsoMetaCell.isoBuildings of all cells.
    pub iso_building_by_id: std::collections::HashMap<i64, usize>,
}

impl IsoMetaGrid {
    pub fn room(&self, r: RoomRef) -> &RoomDef {
        &self.room_defs[r.0 as usize]
    }
    pub fn building(&self, b: BuildingRef) -> &BuildingDef {
        &self.building_defs[b.0 as usize]
    }

    /// zombie.iso.IsoMetaGrid.getLotDirectories (String, ArrayList): the map folder and, recursively, the folders its
    /// map.info names in `lots=`.
    pub fn get_lot_directories(media: &Path, map_name: &str, result: &mut Vec<String>) {
        if !result.iter().any(|r| r == map_name) {
            if let Some(map_info) = choose_game_info::get_map_details(media, map_name) {
                result.push(map_name.to_string());
                for lot_dir in &map_info.lots_dir {
                    Self::get_lot_directories(media, lot_dir, result);
                }
            }
        }
    }

    /// zombie.iso.IsoMetaGrid.getLotDirectories (): `game_map` is `Core.gameMap`, either one map name or a `;` list.
    pub fn lot_directories(media: &Path, game_map: &str) -> Vec<String> {
        let mut result = Vec::new();
        if game_map.contains(';') {
            for ss in game_map.split(';') {
                let lot_dir = java::trim(ss);
                if !lot_dir.is_empty() && !result.iter().any(|r| r == lot_dir) {
                    result.push(lot_dir.to_string());
                }
            }
        } else {
            Self::get_lot_directories(media, game_map, &mut result);
        }
        result
    }

    /// zombie.iso.IsoMetaGrid.CreateStep1, up to where the loader threads start: index the map folders and size the grid.
    /// `world_gen_bounds` is (minXCell, minYCell, maxXCell, maxYCell) of `WorldGenParams` (default -250..250).
    pub fn create_step1(
        &mut self,
        media: &Path,
        lot_dirs: &[String],
        world_gen_bounds: (i32, i32, i32, i32),
    ) -> Result<(), String> {
        self.min_x = 10000000;
        self.min_y = 10000000;
        self.max_x = -10000000;
        self.max_y = -10000000;
        self.map_files.clear();
        for (i, lot_dir) in lot_dirs.iter().enumerate() {
            let path = media.join("maps").join(lot_dir);
            if path.is_dir() {
                self.map_files.push(MapFiles::new(lot_dir, &path, i));
            }
        }
        for map_files in &mut self.map_files {
            let fixed2x = choose_game_info::get_map_details(media, &map_files.map_directory_name)
                .ok_or("java.lang.NullPointerException: mapInfo")?
                .fixed2x;
            if map_files.load(fixed2x)? {
                map_files.post_load();
            }
        }
        for m in self.map_files.iter().rev() {
            self.min_x = self.min_x.min(m.min_x);
            self.min_y = self.min_y.min(m.min_y);
            self.max_x = self.max_x.max(m.max_x);
            self.max_y = self.max_y.max(m.max_y);
        }
        if self.min_x > self.max_x {
            self.min_x = 0;
            self.min_y = 0;
            self.max_x = 1;
            self.max_y = 1;
        }
        self.min_non_procedural_x = self.min_x;
        self.min_non_procedural_y = self.min_y;
        self.max_non_procedural_x = self.max_x;
        self.max_non_procedural_y = self.max_y;
        self.min_x = self.min_x.min(world_gen_bounds.0);
        self.min_y = self.min_y.min(world_gen_bounds.1);
        self.max_x = self.max_x.max(world_gen_bounds.2);
        self.max_y = self.max_y.max(world_gen_bounds.3);
        self.width = self.max_x - self.min_x + 1;
        self.height = self.max_y - self.min_y + 1;
        self.grid = (0..self.width as usize * self.height as usize)
            .map(|_| None)
            .collect();
        Ok(())
    }

    fn cell_index(&self, x: i32, y: i32) -> Option<usize> {
        let (gx, gy) = (x - self.min_x, y - self.min_y);
        (gx >= 0 && gy >= 0 && gx < self.width && gy < self.height)
            .then(|| (gx * self.height + gy) as usize)
    }
    /// zombie.iso.IsoMetaGrid.getCellData: by cell coordinates.
    pub fn get_cell_data(&self, x: i32, y: i32) -> Option<&IsoMetaCell> {
        self.cell_index(x, y).and_then(|i| self.grid[i].as_deref())
    }

    /// zombie.iso.IsoMetaGrid.higherPriority300x300CellExists
    pub fn higher_priority_300x300_cell_exists(
        &self,
        priority: usize,
        cell300_x: i32,
        cell300_y: i32,
    ) -> bool {
        self.map_files[..priority]
            .iter()
            .any(|m| m.has_cell300(cell300_x, cell300_y))
    }

    /// zombie.iso.IsoLot.getHeader: the header of the highest-priority folder that has the cell.
    pub fn get_header(&self, cell_x: i32, cell_y: i32) -> Option<&LotHeader> {
        let name = format!("{cell_x}_{cell_y}.lotheader");
        self.map_files
            .iter()
            .find_map(|m| m.info_headers.get(&name))
    }

    /// The loader threads of `CreateStep1`, run one after another: every cell of every map folder, rows first.
    /// Errors are collected as the original logs them ("ERROR loading <file>") and loading goes on.
    pub fn load_all_cells(
        &mut self,
        rand_next: &mut dyn FnMut(i32) -> i32,
        voronoi_values: &mut dyn FnMut(i32, i32) -> Vec<Vec<f64>>,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        for w_y in self.min_y..=self.max_y {
            for w_x in self.min_x..=self.max_x {
                for priority in 0..self.map_files.len() {
                    let Some(path) = self.map_files[priority]
                        .info_file_names
                        .get(&format!("{w_x}_{w_y}.lotheader"))
                        .cloned()
                    else {
                        continue;
                    };
                    let Ok(data) = std::fs::read(&path) else {
                        continue;
                    };
                    let v = voronoi_values(w_x, w_y);
                    if let Err(e) = self.load_cell(priority, &data, w_x, w_y, rand_next, &v) {
                        errors.push(format!("ERROR loading {}: {e}", path.display()));
                    }
                }
            }
        }
        errors
    }

    /// zombie.iso.IsoMetaGrid.MetaGridLoaderThread.loadCell (MapFiles, int, int): reads one lotheader (tile names, size,
    /// levels, zombie intensity) and appends its rooms and buildings to the cell.
    ///
    /// `rand_next` is `Rand.Next(int)` (one draw of 4 per room, deciding `lightsActive`). `voronoi_values` are the
    /// `ZombieVoronoi.evaluateCellCutoff` tables multiplied into the intensities; pass none until world generation is
    /// ported. `Err` is the exception the original logs before moving on: whatever was stored before it stays.
    pub fn load_cell(
        &mut self,
        priority: usize,
        data: &[u8],
        w_x: i32,
        w_y: i32,
        rand_next: &mut dyn FnMut(i32) -> i32,
        voronoi_values: &[Vec<f64>],
    ) -> Result<(), String> {
        let name = format!("{w_x}_{w_y}.lotheader");
        let Some(mut info) = self.map_files[priority].info_headers.remove(&name) else {
            return Ok(());
        };
        let cell_idx = self
            .cell_index(w_x, w_y)
            .ok_or("java.lang.ArrayIndexOutOfBoundsException: cell outside the grid")?;
        if self.grid[cell_idx].is_none() {
            self.grid[cell_idx] = Some(Box::new(IsoMetaCell::new(w_x, w_y, priority)));
        }
        let r = self.load_cell_inner(
            priority,
            &mut info,
            cell_idx,
            data,
            w_x,
            w_y,
            rand_next,
            voronoi_values,
        );
        self.map_files[priority].info_headers.insert(name, info);
        r
    }

    #[allow(clippy::too_many_arguments)] // the original reads these from the thread, the grid and globals
    fn load_cell_inner(
        &mut self,
        priority: usize,
        info: &mut LotHeader,
        cell_idx: usize,
        data: &[u8],
        w_x: i32,
        w_y: i32,
        rand_next: &mut dyn FnMut(i32) -> i32,
        voronoi_values: &[Vec<f64>],
    ) -> Result<(), String> {
        let mut input = RandomAccess::new(data);
        if input.read_bytes(4) != LOTHEADER_MAGIC {
            input.seek(0);
        }
        info.version = iso_lot::read_int(&mut input)?;
        if info.version < 0 || info.version > 1 {
            return Err(format!(
                "java.io.IOException: Unsupported version {}",
                info.version
            ));
        }
        let tilecount = iso_lot::read_int(&mut input)?;
        for _ in 0..tilecount {
            let s = iso_lot::read_string(&mut input)
                .ok_or("java.lang.NullPointerException: tile name")?;
            info.tiles_used.push(java::trim(&s).to_string());
        }
        if info.version == 0 {
            input.read();
        }
        info.width = iso_lot::read_int(&mut input)?;
        info.height = iso_lot::read_int(&mut input)?;
        if info.width != 8 || info.height != info.width {
            return Err(format!(
                "java.io.IOException: invalid chunk width {}x{}",
                info.width, info.height
            ));
        }
        if info.version == 0 {
            info.min_level = 0;
            info.max_level = iso_lot::read_int(&mut input)?.wrapping_sub(1);
        } else {
            info.min_level = iso_lot::read_int(&mut input)?;
            info.max_level = iso_lot::read_int(&mut input)?;
        }

        // `this.roomList` of the loader: the rooms of this file only, which the building records index into.
        let mut room_list: Vec<RoomDef> = Vec::new();
        let num_rooms = iso_lot::read_int(&mut input)?;
        for _ in 0..num_rooms {
            let name = iso_lot::read_string(&mut input)
                .ok_or("java.lang.NullPointerException: room name")?;
            let mut room_def = RoomDef::new(0, name);
            room_def.level = iso_lot::read_int(&mut input)?;
            let rects = iso_lot::read_int(&mut input)?;
            for _ in 0..rects {
                let rx = iso_lot::read_int(&mut input)?;
                let ry = iso_lot::read_int(&mut input)?;
                let rw = iso_lot::read_int(&mut input)?;
                let rh = iso_lot::read_int(&mut input)?;
                room_def.rects.push(RoomRect {
                    x: rx.wrapping_add(w_x * 256),
                    y: ry.wrapping_add(w_y * 256),
                    w: rw,
                    h: rh,
                });
            }
            room_def.calculate_bounds();
            let n_objects = iso_lot::read_int(&mut input)?;
            for _ in 0..n_objects {
                let e = iso_lot::read_int(&mut input)?;
                let x = iso_lot::read_int(&mut input)?;
                let y = iso_lot::read_int(&mut input)?;
                room_def.objects.push(MetaObject::new(
                    e,
                    x.wrapping_add(w_x * 256).wrapping_sub(room_def.x),
                    y.wrapping_add(w_y * 256).wrapping_sub(room_def.y),
                ));
            }
            room_def.lights_active = rand_next(4) == 0;
            room_list.push(room_def);
        }

        // The rooms move into the cell before buildings are read; if a later read fails they stay there, as in the original.
        let base = self.room_defs.len();
        let file_rooms = room_list.len();
        for mut room_def in room_list {
            let cell = self.grid[cell_idx].as_mut().ok_or("cell")?;
            let r = RoomRef(self.room_defs.len() as u32);
            room_def.id = room_id::make_id(w_x, w_y, cell.room_list.len() as i32);
            room_def.meta_id = room_def.calculate_meta_id(w_x, w_y);
            cell.rooms.insert(room_def.id, r);
            cell.room_by_meta_id.insert(room_def.meta_id, r);
            cell.room_list.push(r);
            self.room_defs.push(room_def);
        }

        let num_buildings = iso_lot::read_int(&mut input)?;
        for n in 0..num_buildings {
            let mut building_def = BuildingDef::default();
            let b = BuildingRef(self.building_defs.len() as u32);
            let numb_rooms = iso_lot::read_int(&mut input)?;
            building_def.id = building_id::make_id(w_x, w_y, n);
            for _ in 0..numb_rooms {
                let room_index = iso_lot::read_int(&mut input)?;
                if room_index < 0 || room_index as usize >= file_rooms {
                    return Err(format!("java.lang.IndexOutOfBoundsException: Index {room_index} out of bounds for length {file_rooms}"));
                }
                let r = base + room_index as usize;
                self.room_defs[r].building = Some(b);
                if self.room_defs[r].is_empty_outside() {
                    building_def.emptyoutside.push(RoomRef(r as u32));
                } else {
                    building_def.rooms.push(RoomRef(r as u32));
                }
            }
            building_def.calculate_bounds(&self.room_defs);
            let cell300_x = building_def.x / 300;
            let cell300_y = building_def.y / 300;
            let listed = !self.higher_priority_300x300_cell_exists(priority, cell300_x, cell300_y);
            if listed {
                building_def.meta_id = building_def.calculate_meta_id(&self.room_defs, w_x, w_y);
                let cell = self.grid[cell_idx].as_mut().ok_or("cell")?;
                cell.buildings.push(b);
                cell.building_by_meta_id.insert(building_def.meta_id, b);
            }
            // A building hidden by a higher-priority folder is still referenced by its rooms.
            self.building_defs.push(building_def);
        }

        let zombie_intensity = input.read_bytes(1024);
        if zombie_intensity.len() != 1024 {
            return Err(format!("java.io.EOFException: wx={w_x}, wy={w_y}, nBytes={}, this.zombieIntensity.length=1024", zombie_intensity.len()));
        }
        for wx in 0..32usize {
            for wy in 0..32usize {
                // The file stores column-major (x * 32 + y); LotHeader keeps row-major.
                let mut intensity = zombie_intensity[wx * 32 + wy] as i32;
                let mut v = 1.0f64;
                for values in voronoi_values {
                    v *= values[wy * 32 + wx];
                }
                intensity = (intensity as f64 * v) as i32;
                intensity = pz_math::clamp_i32(intensity, 0, 255);
                info.set_zombie_intensity(wx + wy * 32, intensity as u8);
                let (cx, cy) = (wx as i32, wy as i32);
                if self.grid[cell_idx]
                    .as_ref()
                    .is_some_and(|c| c.has_chunk(cx, cy))
                    && !self.higher_priority_300x300_cell_exists(
                        priority,
                        (w_x * 256 + cx * 8) / 300,
                        (w_y * 256 + cy * 8) / 300,
                    )
                {
                    if let Some(c) = self.grid[cell_idx].as_mut() {
                        c.get_chunk(wy * 32 + wx, || 0)
                            .set_zombie_intensity(intensity as u8);
                    }
                }
            }
        }
        Ok(())
    }

    /// zombie.iso.LotHeader.getZombieIntensityForChunk for the header of folder `priority` at this cell: the value from
    /// the first folder, starting there, whose 300-cell grid covers the chunk; -1 if none.
    pub fn get_zombie_intensity_for_chunk(
        &self,
        priority: usize,
        cell_x: i32,
        cell_y: i32,
        chunk_x: i32,
        chunk_y: i32,
    ) -> i32 {
        if !(0..32).contains(&chunk_x) || !(0..32).contains(&chunk_y) {
            return -1;
        }
        for m in &self.map_files[priority.min(self.map_files.len())..] {
            let cell300_x = pz_math::fastfloor((cell_x * 256 + chunk_x * 8) as f32 / 300.0);
            let cell300_y = pz_math::fastfloor((cell_y * 256 + chunk_y * 8) as f32 / 300.0);
            if m.bg_has_cell300
                .get_value(cell300_x - m.min_cell300_x, cell300_y - m.min_cell300_y)
            {
                // The original dereferences the header without a null check here.
                return m.get_lot_header(cell_x, cell_y).map_or(-1, |h| {
                    h.get_zombie_intensity((chunk_x + chunk_y * 32) as usize) as i32
                });
            }
        }
        -1
    }

    /// zombie.iso.IsoMetaCell.addRoom on the cell at (cell_x, cell_y), creating meta chunks as the original does.
    fn add_room_to_cell(&mut self, cell_x: i32, cell_y: i32, r: RoomRef) {
        let Some(idx) = self.cell_index(cell_x, cell_y) else {
            return;
        };
        let Some(mut cell) = self.grid[idx].take() else {
            return;
        };
        let priority = cell.info_priority;
        cell.add_room(
            r,
            &self.room_defs[r.0 as usize],
            cell_x * 256,
            cell_y * 256,
            |cx, cy| self.get_zombie_intensity_for_chunk(priority, cell_x, cell_y, cx, cy),
        );
        self.grid[idx] = Some(cell);
    }

    /// zombie.iso.IsoMetaGrid.consolidateBuildings: collects every visible building into `buildings` and registers its
    /// rooms in the meta chunks of its own cell. Headers are walked in the original's `HashMap` order.
    fn consolidate_buildings(&mut self) {
        for i in 0..self.map_files.len() {
            let names: Vec<&str> = self.map_files[i]
                .info_header_names
                .iter()
                .map(String::as_str)
                .collect();
            let cells: Vec<(i32, i32)> = java::hash_map_order(&names)
                .into_iter()
                .filter_map(|k| {
                    self.map_files[i]
                        .info_headers
                        .get(names[k])
                        .map(|h| (h.cell_x, h.cell_y))
                })
                .collect();
            for (cell_x, cell_y) in cells {
                // The original dereferences the cell without a null check; a header that failed to load has none.
                let Some(cell_buildings) = self
                    .get_cell_data(cell_x, cell_y)
                    .map(|c| c.buildings.clone())
                else {
                    continue;
                };
                for b in cell_buildings {
                    let def = &self.building_defs[b.0 as usize];
                    if self.higher_priority_300x300_cell_exists(i, def.x / 300, def.y / 300) {
                        continue;
                    }
                    self.buildings.push(b);
                    let rooms: Vec<RoomRef> =
                        def.rooms.iter().chain(&def.emptyoutside).copied().collect();
                    for r in rooms {
                        self.add_room_to_cell(cell_x, cell_y, r);
                    }
                }
            }
        }
    }

    /// zombie.iso.IsoMetaGrid.initIncompleteCells: for cells not aligned to the old 300-square grid, which neighbours exist.
    fn init_incomplete_cells(&mut self) {
        for m in &mut self.map_files {
            for cell_y in m.min_y..=m.max_y {
                for cell_x in m.min_x..=m.max_x {
                    if m.get_lot_header(cell_x, cell_y).is_none() {
                        continue;
                    }
                    let mut adj = [false; 8];
                    let (mx, my) = (
                        pz_math::coordmodulo(cell_x * 256, 300) > 0,
                        pz_math::coordmodulo(cell_y * 256, 300) > 0,
                    );
                    let mut set = |d: IsoDirections, v: bool| adj[d.ordinal()] = v;
                    if mx {
                        set(IsoDirections::W, m.has_cell(cell_x - 1, cell_y));
                        set(IsoDirections::E, m.has_cell(cell_x + 1, cell_y));
                    }
                    if my {
                        set(IsoDirections::N, m.has_cell(cell_x, cell_y - 1));
                        set(IsoDirections::S, m.has_cell(cell_x, cell_y + 1));
                    }
                    if mx && my {
                        set(IsoDirections::NW, m.has_cell(cell_x - 1, cell_y - 1));
                        set(IsoDirections::NE, m.has_cell(cell_x + 1, cell_y - 1));
                        set(IsoDirections::SE, m.has_cell(cell_x + 1, cell_y + 1));
                        set(IsoDirections::SW, m.has_cell(cell_x - 1, cell_y + 1));
                    }
                    if let Some(h) = m
                        .info_headers
                        .get_mut(&format!("{cell_x}_{cell_y}.lotheader"))
                    {
                        h.adjacent_cells = adj;
                    }
                }
            }
        }
    }

    /// zombie.iso.IsoMetaGrid.addRoomsToAdjacentCells (BuildingDef): a building that crosses a cell border also
    /// registers its rooms in the cells to the east, south and south-east of its origin cell.
    pub fn add_rooms_to_adjacent_cells(&mut self, b: BuildingRef) {
        let def = &self.building_defs[b.0 as usize];
        let (cell_x, cell_y) = (def.x / 256, def.y / 256);
        let (cell_x2, cell_y2) = ((def.x2 - 1) / 256, (def.y2 - 1) / 256);
        if cell_x2 == cell_x && cell_y2 == cell_y {
            return;
        }
        let (rooms, empty) = (def.rooms.clone(), def.emptyoutside.clone());
        for dy in 0..=1 {
            for dx in 0..=1 {
                if dx != 0 || dy != 0 {
                    for &r in rooms.iter().chain(&empty) {
                        self.add_room_to_cell(cell_x + dx, cell_y + dy, r);
                    }
                }
            }
        }
    }

    /// zombie.iso.IsoMetaGrid.CreateStep2 after the loader threads finish: building list, incomplete cells, rooms of
    /// border-crossing buildings, and which buildings have a burglar alarm (`rand_next` is `Rand.Next(int)`,
    /// `rand_range` is `Rand.Next(int, int)`).
    pub fn create_step2(
        &mut self,
        alarm: &AlarmSettings,
        rand_next: &mut dyn FnMut(i32) -> i32,
        rand_range: &mut dyn FnMut(i32, i32) -> i32,
    ) {
        self.consolidate_buildings();
        self.init_incomplete_cells();
        for ixx in 0..self.buildings.len() {
            let b = self.buildings[ixx];
            self.add_rooms_to_adjacent_cells(b);
            let def = &mut self.building_defs[b.0 as usize];
            if !alarm.last_stand && def.rooms.len() > 2 {
                let mut rand_alarm = 11;
                if alarm.power_grid_exists {
                    rand_alarm = 9;
                }
                match alarm.alarm {
                    1 => rand_alarm = -1,
                    2 => rand_alarm += 5,
                    3 => rand_alarm += 3,
                    5 => rand_alarm -= 3,
                    6 => rand_alarm -= 5,
                    _ => {}
                }
                if rand_alarm > -1 {
                    def.alarmed = rand_next(rand_alarm) == 0;
                }
                if def.alarmed {
                    // zombie.SandboxOptions.randomAlarmDecay
                    def.alarm_decay = match alarm.alarm_decay {
                        2 => rand_range(0, 30),
                        3 => rand_range(0, 60),
                        4 => rand_range(0, 180),
                        5 => rand_range(0, 360),
                        6 => rand_range(0, 1800),
                        _ => 0,
                    };
                }
            }
        }
    }

    /// zombie.iso.IsoMetaGrid.getRoomByID: the runtime room for a room id, created on first use together with its
    /// building and, through `IsoBuilding.CreateFrom`, the building's other rooms. A new `IsoBuilding` draws
    /// `Rand.Next(120)` once (`rand_next`). Returns an index into `iso_rooms`.
    pub fn get_room_by_id(
        &mut self,
        room_id: i64,
        rand_next: &mut dyn FnMut(i32) -> i32,
    ) -> Option<usize> {
        let (cell_x, cell_y) = (room_id::get_cell_x(room_id), room_id::get_cell_y(room_id));
        let r = *self.get_cell_data(cell_x, cell_y)?.rooms.get(&room_id)?;
        if let Some(i) = self.iso_room_by_id.get(&room_id) {
            return Some(*i);
        }
        let b = match self.room_defs[r.0 as usize].building {
            Some(b) => b,
            None => {
                let mut def = BuildingDef {
                    id: building_id::make_id(cell_x, cell_y, self.buildings.len() as i32),
                    ..BuildingDef::default()
                };
                def.rooms.push(r);
                def.calculate_bounds(&self.room_defs);
                def.meta_id = def.calculate_meta_id(&self.room_defs, cell_x, cell_y);
                let b = BuildingRef(self.building_defs.len() as u32);
                self.building_defs.push(def);
                self.room_defs[r.0 as usize].building = Some(b);
                self.buildings.push(b);
                b
            }
        };
        let building_id = self.building_defs[b.0 as usize].id;
        let room = self.iso_rooms.len();
        self.iso_rooms.push(IsoRoom {
            def: r,
            building: usize::MAX,
        });
        self.iso_room_by_id.insert(room_id, room);
        match self.iso_building_by_id.get(&building_id).copied() {
            None => {
                let ib = self.iso_buildings.len();
                // zombie.iso.areas.IsoBuilding constructor
                self.iso_buildings.push(IsoBuilding {
                    id: ib as i32,
                    score_update: -120 + rand_next(120),
                    def: b,
                    rooms: Vec::new(),
                });
                self.iso_rooms[room].building = ib;
                self.iso_building_by_id.insert(building_id, ib);
                // zombie.iso.areas.IsoBuilding.CreateFrom
                for n in 0..self.building_defs[b.0 as usize].rooms.len() {
                    let other = self.building_defs[b.0 as usize].rooms[n];
                    let other_id = self.room_defs[other.0 as usize].id;
                    if let Some(rr) = self.get_room_by_id(other_id, rand_next) {
                        self.iso_rooms[rr].building = ib;
                        if !self.iso_buildings[ib].rooms.contains(&rr) {
                            self.iso_buildings[ib].rooms.push(rr);
                        }
                    }
                }
            }
            Some(ib) => {
                self.iso_rooms[room].building = ib;
                self.iso_buildings[ib].rooms.push(room);
            }
        }
        Some(room)
    }

    /// zombie.iso.IsoMetaGrid.getChunkData: the meta chunk if it exists. The original creates it on demand; a chunk
    /// created that way has no rooms, so lookups through this method see the same rooms.
    pub fn get_chunk_data(
        &self,
        chunk_x: i32,
        chunk_y: i32,
    ) -> Option<&super::iso_meta_chunk::IsoMetaChunk> {
        let cell_x = pz_math::fastfloor(chunk_x as f32 / 32.0);
        let cell_y = pz_math::fastfloor(chunk_y as f32 / 32.0);
        let (lx, ly) = (chunk_x - cell_x * 32, chunk_y - cell_y * 32);
        if !(0..32).contains(&lx) || !(0..32).contains(&ly) {
            return None;
        }
        self.get_cell_data(cell_x, cell_y)?
            .chunk((ly * 32 + lx) as usize)
    }

    /// zombie.iso.IsoMetaGrid.getChunkDataFromTile
    pub fn get_chunk_data_from_tile(
        &self,
        x: i32,
        y: i32,
    ) -> Option<&super::iso_meta_chunk::IsoMetaChunk> {
        self.get_chunk_data(
            pz_math::fastfloor(x as f32 / 8.0),
            pz_math::fastfloor(y as f32 / 8.0),
        )
    }

    /// zombie.iso.IsoMetaGrid.getRoomAt
    pub fn get_room_at(&self, x: i32, y: i32, z: i32) -> Option<RoomRef> {
        self.get_chunk_data_from_tile(x, y)?
            .get_room_at(&self.room_defs, x, y, z)
    }

    /// zombie.iso.IsoMetaGrid.getEmptyOutsideAt
    pub fn get_empty_outside_at(&self, x: i32, y: i32, z: i32) -> Option<RoomRef> {
        self.get_chunk_data_from_tile(x, y)?
            .get_empty_outside_at(&self.room_defs, x, y, z)
    }
}
