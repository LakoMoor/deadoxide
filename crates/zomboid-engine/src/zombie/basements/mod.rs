//! zombie.basements.Basements: procedural basements. Before the world's meta data is final, every "Basement" zone of
//! the map gets one of the basement layouts (`media/binmap/*.pzby`) that fits, chosen with the world seed, and the
//! layout's rooms are added to the meta grid, merged into the house above where they touch it.
use std::path::Path;

use crate::java::{self, Random};
use crate::zombie::core::random::rand_abstract::RandAbstract;
use crate::zombie::iso::building_def::BuildingDef;
use crate::zombie::iso::building_id;
use crate::zombie::iso::iso_lot::{self, RandomAccess};
use crate::zombie::iso::iso_meta_grid::{BuildingRef, IsoMetaGrid, RoomRef};
use crate::zombie::iso::room_def::{RoomDef, RoomRect};
use crate::zombie::iso::room_id;

/// zombie.iso.NewMapBinaryFile.Header: rooms and buildings of a `.pzby` file, in file-local coordinates.
#[derive(Clone, Debug, Default)]
pub struct Header {
    pub version: i32,
    pub width: i32,
    pub height: i32,
    pub levels: i32,
    pub used_tile_names: Vec<String>,
    pub room_def_list: Vec<RoomDef>,
    /// Buildings; their room handles index `room_def_list`.
    pub building_def_list: Vec<BuildingDef>,
}

/// zombie.iso.NewMapBinaryFile.loadHeaderInternal: `Err` is the IOException (or other exception) of the original.
pub fn load_header(data: &[u8]) -> Result<Header, String> {
    let mut input = RandomAccess::new(data);
    if input.read_bytes(4) != b"PZBY" {
        return Err("java.io.IOException: unrecognized file format".into());
    }
    let mut header = Header {
        version: iso_lot::read_int(&mut input)?,
        ..Header::default()
    };
    let num_tile_names = iso_lot::read_int(&mut input)?;
    for _ in 0..num_tile_names {
        let s =
            iso_lot::read_string(&mut input).ok_or("java.lang.NullPointerException: tile name")?;
        header.used_tile_names.push(java::trim(&s).to_string());
    }
    header.width = iso_lot::read_int(&mut input)?;
    header.height = iso_lot::read_int(&mut input)?;
    header.levels = iso_lot::read_int(&mut input)?;
    let num_rooms = iso_lot::read_int(&mut input)?;
    for n in 0..num_rooms {
        let name =
            iso_lot::read_string(&mut input).ok_or("java.lang.NullPointerException: room name")?;
        let mut room_def = RoomDef::new(n as i64, name);
        room_def.level = iso_lot::read_int(&mut input)?;
        let num_room_rects = iso_lot::read_int(&mut input)?;
        for _ in 0..num_room_rects {
            let x = iso_lot::read_int(&mut input)?;
            let y = iso_lot::read_int(&mut input)?;
            let w = iso_lot::read_int(&mut input)?;
            let h = iso_lot::read_int(&mut input)?;
            room_def.rects.push(RoomRect { x, y, w, h });
        }
        room_def.calculate_bounds();
        let n_objects = iso_lot::read_int(&mut input)?;
        for _ in 0..n_objects {
            // Meta objects of basement files are read and, for now, dropped.
            for _ in 0..3 {
                iso_lot::read_int(&mut input)?;
            }
        }
        // The original draws Rand.Next(2) for lightsActive here; this header copy is never placed in the world.
        header.room_def_list.push(room_def);
    }
    let num_buildings = iso_lot::read_int(&mut input)?;
    for n in 0..num_buildings {
        let mut building_def = BuildingDef {
            id: n as i64,
            ..BuildingDef::default()
        };
        let num_rooms = iso_lot::read_int(&mut input)?;
        for _ in 0..num_rooms {
            let id = iso_lot::read_int(&mut input)?;
            let rr = usize::try_from(id)
                .ok()
                .filter(|i| *i < header.room_def_list.len())
                .ok_or("java.lang.NullPointerException: room of building")?;
            if header.room_def_list[rr].is_empty_outside() {
                building_def.emptyoutside.push(RoomRef(rr as u32));
            } else {
                building_def.rooms.push(RoomRef(rr as u32));
            }
        }
        building_def.calculate_bounds(&header.room_def_list);
        header.building_def_list.push(building_def);
    }
    Ok(header)
}

/// zombie.basements.BasementDefinition
#[derive(Clone, Debug, Default)]
pub struct BasementDefinition {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub stairx: i32,
    pub stairy: i32,
    pub north: bool,
    pub header: Option<Header>,
}

/// zombie.basements.BasementSpawnLocation
#[derive(Clone, Debug, Default)]
pub struct BasementSpawnLocation {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub w: i32,
    pub h: i32,
    pub stair_x: i32,
    pub stair_y: i32,
    pub north: bool,
    pub specific_basement: Option<Vec<String>>,
    pub access: Option<String>,
}

/// zombie.basements.BasementPlacement
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BasementPlacement {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub w: i32,
    pub h: i32,
}

/// zombie.basements.BasementsPerMap: what one map folder registered through the Lua API.
#[derive(Clone, Debug, Default)]
pub struct BasementsPerMap {
    pub map_id: String,
    pub basement_definitions: Vec<BasementDefinition>,
    pub basement_access_definitions: Vec<BasementDefinition>,
    pub basement_spawn_locations: Vec<BasementSpawnLocation>,
}

/// `key = value` pairs of one Lua table constructor line, without evaluating Lua: enough for the flat tables of
/// `basements.lua` and `objects.lua`.
fn lua_fields(text: &str) -> Vec<(String, String)> {
    text.split([',', '{', '}'])
        .filter_map(|t| t.split_once('='))
        .map(|(k, v)| {
            (
                k.trim().to_string(),
                v.trim().trim_matches(['"', '\'']).to_string(),
            )
        })
        .collect()
}
fn field<'a>(f: &'a [(String, String)], k: &str) -> Option<&'a str> {
    f.iter().find(|p| p.0 == k).map(|p| p.1.as_str())
}
/// `((Double) table.rawget(k)).intValue()`
fn int(f: &[(String, String)], k: &str) -> Result<i32, String> {
    field(f, k)
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v as i32)
        .ok_or_else(|| format!("java.lang.NullPointerException: {k}"))
}

impl BasementsPerMap {
    /// What running `media/maps/<map>/basements.lua` and the "Basement" zones of `objects.lua` registers through
    /// zombie.basements.BasementsV1 (addAccessDefinitions, addBasementDefinitions, registerBasementSpawnLocation).
    /// The two files are read as data here, not executed: tables are taken in source order (Kahlua tables iterate in
    /// insertion order), commented lines are skipped, and the `api:add...` calls are assumed to be the vanilla ones.
    /// The commented-out `procedural_basement_spawn_locations` list of the vanilla file is empty and not read.
    pub fn load(media: &Path, map_id: &str) -> Result<Self, String> {
        let mut m = BasementsPerMap {
            map_id: map_id.to_string(),
            ..Default::default()
        };
        let dir = media.join("maps").join(map_id);
        if let Ok(text) = std::fs::read_to_string(dir.join("basements.lua")) {
            let mut section = "";
            for line in text.lines() {
                let l = line.trim();
                if l.starts_with("--") || l.is_empty() {
                    continue;
                }
                if let Some(rest) = l.strip_prefix("local ") {
                    section = if rest.starts_with("procedural_basements ") {
                        "basements"
                    } else if rest.starts_with("procedural_basement_access ") {
                        "access"
                    } else {
                        ""
                    };
                    continue;
                }
                if l == "}" {
                    section = "";
                    continue;
                }
                let Some((name, body)) = l.split_once('=') else {
                    continue;
                };
                if section.is_empty() || !body.trim_start().starts_with('{') {
                    continue;
                }
                let f = lua_fields(body);
                let def = BasementDefinition {
                    name: name.trim().to_string(),
                    width: int(&f, "width")?,
                    height: int(&f, "height")?,
                    stairx: int(&f, "stairx")?,
                    stairy: int(&f, "stairy")?,
                    north: field(&f, "stairDir") == Some("N"),
                    header: None,
                };
                // basementDefinitionByName.put replaces an earlier definition of the same name in lookups; the list keeps both.
                if section == "basements" {
                    m.basement_definitions.push(def);
                } else {
                    m.basement_access_definitions.push(def);
                }
            }
        }
        if let Ok(text) = std::fs::read_to_string(dir.join("objects.lua")) {
            // Entries start with "{ name" at two spaces of indentation and may continue on following lines.
            let mut entries: Vec<String> = Vec::new();
            for line in text.lines() {
                if line.starts_with("  { ") {
                    entries.push(line.to_string());
                } else if line.starts_with("    ") || line.starts_with("  }") {
                    if let Some(e) = entries.last_mut() {
                        e.push_str(line);
                    }
                }
            }
            for e in entries {
                if !e.contains("type = \"Basement\"") {
                    continue;
                }
                let f = lua_fields(&e);
                let mut bsl = BasementSpawnLocation {
                    x: int(&f, "x")?,
                    y: int(&f, "y")?,
                    z: int(&f, "z")?,
                    w: int(&f, "width")?,
                    h: int(&f, "height")?,
                    north: true,
                    ..Default::default()
                };
                if let Some(v) = field(&f, "StairX").and_then(|v| v.parse::<f64>().ok()) {
                    bsl.stair_x = v as i32;
                }
                if let Some(v) = field(&f, "StairY").and_then(|v| v.parse::<f64>().ok()) {
                    bsl.stair_y = v as i32;
                }
                if let Some(v) = field(&f, "StairDirection") {
                    bsl.north = v == "N";
                }
                bsl.access = field(&f, "Access").map(str::to_string);
                m.basement_spawn_locations.push(bsl);
            }
        }
        Ok(m)
    }
}

/// A placed basement as zombie.basements.BasementOverlap sees it: bounding box and level range of the layout's first building.
struct Placed {
    x: i32,
    y: i32,
    z: i32,
    w: i32,
    h: i32,
    max_level: i32,
}

/// zombie.basements.Basements: the state between choosing placements and merging them into the meta grid.
#[derive(Debug, Default)]
pub struct Basements {
    pub per_map: Vec<BasementsPerMap>,
    pub basement_placements: Vec<BasementPlacement>,
    /// zombie.basements.Basements.buildingDefs: basement buildings that stayed separate (not merged into a house).
    pub building_defs: Vec<BuildingRef>,
    /// Problems the original logs.
    pub log: Vec<String>,
}

impl Basements {
    /// zombie.basements.Basements.beforeLoadMetaGrid for a new world (no `map_basements.bin` yet): load the layout
    /// headers, choose placements with `Random(seed)`, create the basement buildings and merge them into `grid`.
    pub fn before_load_meta_grid(
        media: &Path,
        grid: &mut IsoMetaGrid,
        lot_directories: &[String],
        seed: i64,
    ) -> Result<Self, String> {
        let mut b = Basements::default();
        for dir in lot_directories {
            let mut m = BasementsPerMap::load(media, dir)?;
            // loadBasementDefinitionHeaders: a layout must hold exactly one building; its size replaces the Lua numbers.
            for def in &mut m.basement_definitions {
                if let Some(h) =
                    std::fs::read(media.join("binmap").join(format!("{}.pzby", def.name)))
                        .ok()
                        .and_then(|d| load_header(&d).ok())
                        .filter(|h| h.building_def_list.len() == 1)
                {
                    def.width = h.building_def_list[0].get_w();
                    def.height = h.building_def_list[0].get_h();
                    def.header = Some(h);
                }
            }
            for def in &mut m.basement_access_definitions {
                def.header = std::fs::read(
                    media
                        .join("basement_access")
                        .join(format!("{}.pzby", def.name)),
                )
                .ok()
                .and_then(|d| load_header(&d).ok());
            }
            if !m.basement_definitions.is_empty()
                || !m.basement_access_definitions.is_empty()
                || !m.basement_spawn_locations.is_empty()
            {
                b.per_map.push(m);
            }
        }
        b.calculate_basement_placements(grid, seed)?;
        b.create_basement_building_defs(grid);
        b.add_basement_building_defs_to_lot_headers(grid);
        Ok(b)
    }

    /// The definition a name resolves to: `basementDefinitionByName` (a later map folder or definition wins).
    fn by_name<'a>(&'a self, name: &str, access: bool) -> Option<&'a BasementDefinition> {
        self.per_map
            .iter()
            .rev()
            .flat_map(|m| {
                if access {
                    m.basement_access_definitions.iter().rev()
                } else {
                    m.basement_definitions.iter().rev()
                }
            })
            .find(|d| d.name == name)
    }

    /// zombie.basements.Basements.calculateBasementPlacements: per spawn location, the layouts that fit (stairs facing
    /// the same way, inside the zone when the zone is larger than one square, not overlapping an earlier basement),
    /// then one of them picked with the seeded generator. The original walks its per-map table in `HashMap` order;
    /// with a single map folder that is the only order.
    fn calculate_basement_placements(
        &mut self,
        grid: &IsoMetaGrid,
        seed: i64,
    ) -> Result<(), String> {
        let mut rnd = RandAbstract::new(Random::new(seed));
        let mut placed: Vec<Placed> = Vec::new();
        let mut placements = Vec::new();
        for m in &self.per_map {
            for loc in &m.basement_spawn_locations {
                let mut choices: Vec<&BasementDefinition> = Vec::new();
                match &loc.specific_basement {
                    Some(names) if !names.is_empty() => {
                        for name in names {
                            if let Some(def) = m
                                .basement_definitions
                                .iter()
                                .rev()
                                .find(|d| &d.name == name)
                                .filter(|d| d.header.is_some())
                            {
                                if can_place_at(grid, &m.map_id, def, loc, &placed)? {
                                    choices.push(def);
                                }
                            }
                        }
                    }
                    _ => {
                        for def in &m.basement_definitions {
                            if can_place_at(grid, &m.map_id, def, loc, &placed)? {
                                choices.push(def);
                            }
                        }
                    }
                }
                if choices.is_empty() {
                    continue;
                }
                let def = choices[rnd.next(choices.len() as i32) as usize];
                let header = def
                    .header
                    .as_ref()
                    .ok_or("java.lang.NullPointerException: header")?;
                let (x, y) = if loc.w > 1 && loc.h > 1 {
                    (
                        loc.x + loc.stair_x - def.stairx,
                        loc.y + loc.stair_y - def.stairy,
                    )
                } else {
                    (loc.x - def.stairx, loc.y - def.stairy)
                };
                let p = BasementPlacement {
                    name: def.name.clone(),
                    x,
                    y,
                    z: loc.z - header.levels,
                    w: def.width,
                    h: def.height,
                };
                let first = &header.building_def_list[0];
                placed.push(Placed {
                    x: p.x,
                    y: p.y,
                    z: p.z,
                    w: first.get_w(),
                    h: first.get_h(),
                    max_level: first.get_max_level(&header.room_def_list),
                });
                let (px, py) = (p.x, p.y);
                placements.push(p);
                if let Some(access) = &loc.access {
                    if let Some(a) = self.by_name(access, true).filter(|a| a.header.is_some()) {
                        placements.push(BasementPlacement {
                            name: a.name.clone(),
                            x: px + def.stairx - a.stairx,
                            y: py + def.stairy - a.stairy,
                            z: loc.z,
                            w: a.width,
                            h: a.height,
                        });
                    }
                }
            }
        }
        self.basement_placements = placements;
        Ok(())
    }

    /// zombie.basements.Basements.createBasementBuildingDefs: one new building per placement, with copies of the
    /// layout's rooms moved to world coordinates and levels. Rooms and buildings are appended to the grid's arenas.
    fn create_basement_building_defs(&mut self, grid: &mut IsoMetaGrid) {
        for i in 0..self.basement_placements.len() {
            let p = self.basement_placements[i].clone();
            let Some(def) = self
                .by_name(&p.name, false)
                .or_else(|| self.by_name(&p.name, true))
            else {
                continue;
            };
            let Some(header) = def.header.clone() else {
                continue;
            };
            if header.building_def_list.is_empty()
                || grid.get_cell_data(p.x / 256, p.y / 256).is_none()
            {
                continue;
            }
            let b = BuildingRef(grid.building_defs.len() as u32);
            let mut building_def = BuildingDef {
                id: self.building_defs.len() as i64,
                ..BuildingDef::default()
            };
            for room_def in &header.room_def_list {
                let mut new_room =
                    RoomDef::new(building_def.rooms.len() as i64, room_def.name.clone());
                new_room.level = p.z + room_def.level;
                new_room.rects = room_def
                    .rects
                    .iter()
                    .map(|r| RoomRect {
                        x: p.x + r.x,
                        y: p.y + r.y,
                        w: r.w,
                        h: r.h,
                    })
                    .collect();
                new_room.calculate_bounds();
                new_room.building = Some(b);
                let r = RoomRef(grid.room_defs.len() as u32);
                if room_def.is_empty_outside() {
                    building_def.emptyoutside.push(r);
                } else {
                    building_def.rooms.push(r);
                }
                grid.room_defs.push(new_room);
            }
            building_def.calculate_bounds(&grid.room_defs);
            grid.building_defs.push(building_def);
            self.building_defs.push(b);
        }
    }

    /// zombie.basements.Basements.getBuildingToMergeWith: the one above-ground building touching an above-ground
    /// basement part (a basement entrance); none when the part is underground or touches several buildings.
    fn get_building_to_merge_with(grid: &IsoMetaGrid, b: BuildingRef) -> Option<BuildingRef> {
        let def = grid.building(b);
        if def.get_min_level(&grid.room_defs) < 0 {
            return None;
        }
        let mut merge = None;
        for c in
            grid.get_buildings_intersecting(def.x - 1, def.y - 1, def.get_w() + 2, def.get_h() + 2)
        {
            let cand = grid.building(c);
            if cand.get_min_level(&grid.room_defs) >= 0 && def.is_adjacent(cand, &grid.room_defs) {
                if merge.is_some() {
                    return None;
                }
                merge = Some(c);
            }
        }
        merge
    }

    /// zombie.basements.Basements.recalculateBuildingAndRoomIDs: ids follow list positions again.
    fn recalculate_building_and_room_ids(grid: &mut IsoMetaGrid, cell_x: i32, cell_y: i32) {
        let Some(cell) = grid.get_cell_data(cell_x, cell_y) else {
            return;
        };
        let (rooms, buildings) = (cell.room_list.clone(), cell.buildings.clone());
        for (i, r) in rooms.iter().enumerate() {
            grid.room_defs[r.0 as usize].id = room_id::make_id(cell_x, cell_y, i as i32);
        }
        for (i, b) in buildings.iter().enumerate() {
            grid.building_defs[b.0 as usize].id = building_id::make_id(cell_x, cell_y, i as i32);
        }
        let ids: Vec<(i64, RoomRef)> = rooms
            .iter()
            .map(|r| (grid.room_defs[r.0 as usize].id, *r))
            .collect();
        if let Some(cell) = grid.get_cell_data_mut(cell_x, cell_y) {
            cell.rooms = ids.into_iter().collect();
        }
    }

    /// zombie.basements.Basements.mergeRoomsOntoMetaCell: appends rooms to a cell's list with fresh ids and meta ids.
    fn merge_rooms_onto_meta_cell(
        grid: &mut IsoMetaGrid,
        rooms: &[RoomRef],
        cell_x: i32,
        cell_y: i32,
    ) {
        for &r in rooms {
            let Some(n) = grid
                .get_cell_data(cell_x, cell_y)
                .map(|c| c.room_list.len())
            else {
                return;
            };
            let room = &mut grid.room_defs[r.0 as usize];
            room.id = room_id::make_id(cell_x, cell_y, n as i32);
            room.meta_id = room.calculate_meta_id(cell_x, cell_y);
            let (id, meta_id) = (room.id, room.meta_id);
            if let Some(cell) = grid.get_cell_data_mut(cell_x, cell_y) {
                cell.rooms.insert(id, r);
                cell.room_list.push(r);
                cell.room_by_meta_id.insert(meta_id, r);
            }
        }
    }

    /// zombie.basements.Basements.mergeBuildings: the basement part's rooms become rooms of the house `merge`. If the
    /// two lie in different cells, the house first moves to the lower-numbered cell and both cells are renumbered.
    fn merge_buildings(
        grid: &mut IsoMetaGrid,
        b: BuildingRef,
        merge: BuildingRef,
        merged: &mut Vec<(BuildingRef, Vec<RoomRef>, Vec<RoomRef>)>,
    ) {
        let (bd, md) = (grid.building(b), grid.building(merge));
        let (cell_x2, cell_y2) = (md.x / 256, md.y / 256);
        let (cell_x, cell_y) = ((bd.x / 256).min(cell_x2), (bd.y / 256).min(cell_y2));
        if grid.get_cell_data(cell_x, cell_y).is_none() {
            return;
        }
        if cell_x != cell_x2 || cell_y != cell_y2 {
            if grid.get_cell_data(cell_x2, cell_y2).is_none() {
                return;
            }
            // removeBuildingFromMetaCell
            let md = grid.building(merge).clone();
            let gone: Vec<(RoomRef, i64, i64)> = md
                .rooms
                .iter()
                .chain(&md.emptyoutside)
                .map(|r| (*r, grid.room(*r).id, grid.room(*r).meta_id))
                .collect();
            if let Some(c1) = grid.get_cell_data_mut(cell_x2, cell_y2) {
                c1.buildings.retain(|x| *x != merge);
                c1.building_by_meta_id.remove(&md.meta_id);
                for (r, id, meta_id) in &gone {
                    c1.rooms.remove(id);
                    if let Some(i) = c1.room_list.iter().position(|x| x == r) {
                        c1.room_list.remove(i);
                    }
                    c1.room_by_meta_id.remove(meta_id);
                }
            }
            Self::recalculate_building_and_room_ids(grid, cell_x2, cell_y2);
            if let Some(c) = grid.get_cell_data_mut(cell_x, cell_y) {
                c.buildings.push(merge);
                c.room_list.extend(md.rooms.iter().chain(&md.emptyoutside));
            }
            Self::recalculate_building_and_room_ids(grid, cell_x, cell_y);
        }
        let bd = grid.building(b).clone();
        Self::merge_rooms_onto_meta_cell(grid, &bd.rooms, cell_x, cell_y);
        Self::merge_rooms_onto_meta_cell(grid, &bd.emptyoutside, cell_x, cell_y);
        match merged.iter_mut().find(|m| m.0 == merge) {
            Some(m) => {
                m.1.extend(&bd.rooms);
                m.2.extend(&bd.emptyoutside);
            }
            None => merged.push((merge, bd.rooms.clone(), bd.emptyoutside.clone())),
        }
        let old_meta = grid.building(merge).meta_id;
        if let Some(c) = grid.get_cell_data_mut(cell_x, cell_y) {
            c.building_by_meta_id.remove(&old_meta);
        }
        // BuildingDef.addRoomsOf
        for r in bd.rooms.iter().chain(&bd.emptyoutside) {
            grid.room_defs[r.0 as usize].building = Some(merge);
        }
        let mut md = grid.building(merge).clone();
        md.rooms.extend(&bd.rooms);
        md.emptyoutside.extend(&bd.emptyoutside);
        md.calculate_bounds(&grid.room_defs);
        md.meta_id = md.calculate_meta_id(&grid.room_defs, cell_x, cell_y);
        let meta = md.meta_id;
        grid.building_defs[merge.0 as usize] = md;
        if let Some(c) = grid.get_cell_data_mut(cell_x, cell_y) {
            c.building_by_meta_id.insert(meta, merge);
        }
    }

    /// zombie.basements.Basements.addBasementBuildingDefsToLotHeaders followed by addBasementBuildingDefsToMetaGrid:
    /// registers every basement building in its cell (or merges it into a house) and then its rooms in the meta chunks.
    /// The original walks the merged houses in an identity-hash order that differs from run to run; here they are taken
    /// in the order they were first merged. That order only decides the order of rooms inside one meta chunk's list.
    fn add_basement_building_defs_to_lot_headers(&mut self, grid: &mut IsoMetaGrid) {
        let mut merged: Vec<(BuildingRef, Vec<RoomRef>, Vec<RoomRef>)> = Vec::new();
        let mut i = 0;
        while i < self.building_defs.len() {
            let b = self.building_defs[i];
            let (cell_x, cell_y) = (grid.building(b).x / 256, grid.building(b).y / 256);
            if grid.get_cell_data(cell_x, cell_y).is_none() {
                i += 1;
                continue;
            }
            if let Some(merge) = Self::get_building_to_merge_with(grid, b) {
                Self::merge_buildings(grid, b, merge, &mut merged);
                self.building_defs.remove(i);
                continue;
            }
            let index = grid
                .get_cell_data(cell_x, cell_y)
                .map_or(0, |c| c.buildings.len());
            let mut def = grid.building(b).clone();
            def.id = building_id::make_id(cell_x, cell_y, index as i32);
            def.meta_id = def.calculate_meta_id(&grid.room_defs, cell_x, cell_y);
            let (meta, rooms) = (def.meta_id, def.rooms.clone());
            grid.building_defs[b.0 as usize] = def;
            // Only the proper rooms get ids here, as in the original.
            Self::merge_rooms_onto_meta_cell(grid, &rooms, cell_x, cell_y);
            if let Some(c) = grid.get_cell_data_mut(cell_x, cell_y) {
                c.buildings.push(b);
                c.building_by_meta_id.insert(meta, b);
            }
            i += 1;
        }
        for &b in &self.building_defs {
            let def = grid.building(b).clone();
            let (cell_x, cell_y) = (def.x / 256, def.y / 256);
            if grid.get_cell_data(cell_x, cell_y).is_none() {
                continue;
            }
            for r in def.rooms.iter().chain(&def.emptyoutside) {
                grid.add_room_to_cell(cell_x, cell_y, *r);
            }
            grid.add_rooms_to_adjacent_cells(b);
            if !grid.buildings.contains(&b) {
                grid.buildings.push(b);
            }
        }
        for (merge, rooms, empty) in merged {
            let (cell_x, cell_y) = (grid.building(merge).x / 256, grid.building(merge).y / 256);
            if grid.get_cell_data(cell_x, cell_y).is_none() {
                continue;
            }
            for r in rooms.iter().chain(&empty) {
                grid.add_room_to_cell(cell_x, cell_y, *r);
            }
            grid.add_room_list_to_adjacent_cells(merge, &rooms);
            grid.add_room_list_to_adjacent_cells(merge, &empty);
        }
    }
}

/// zombie.basements.Basements.canPlaceAt with BasementOverlap.checkOverlap. The original's cell/chunk index over
/// placed basements only speeds the search up; the test itself is "level ranges overlap and bounding boxes overlap".
fn can_place_at(
    grid: &IsoMetaGrid,
    map_id: &str,
    def: &BasementDefinition,
    loc: &BasementSpawnLocation,
    placed: &[Placed],
) -> Result<bool, String> {
    if def.north != loc.north {
        return Ok(false);
    }
    // A definition whose layout failed to load makes the original throw here.
    let header = def
        .header
        .as_ref()
        .ok_or_else(|| format!("java.lang.NullPointerException: header of {}", def.name))?;
    let (top_x, top_y) = (loc.x + loc.stair_x, loc.y + loc.stair_y);
    let (bx, by, bz) = (
        top_x - def.stairx,
        top_y - def.stairy,
        loc.z - header.levels,
    );
    if loc.w > 1
        && loc.h > 1
        && !(bx >= loc.x
            && bx + def.width <= loc.x + loc.w
            && by >= loc.y
            && by + def.height <= loc.y + loc.h)
    {
        return Ok(false);
    }
    // isCellFromThisMap
    let from_map = grid
        .map_files
        .iter()
        .find(|m| {
            m.info_headers
                .contains_key(&format!("{}_{}.lotheader", loc.x / 256, loc.y / 256))
        })
        .is_some_and(|m| java::equals_ignore_case(&m.map_directory_name, map_id));
    if !from_map {
        return Ok(false);
    }
    let first = &header.building_def_list[0];
    let (w, h, max_level) = (
        first.get_w(),
        first.get_h(),
        first.get_max_level(&header.room_def_list),
    );
    let overlap = placed.iter().any(|p| {
        bz <= p.z + p.max_level
            && bz + max_level >= p.z
            && bx < p.x + p.w
            && bx + w > p.x
            && by < p.y + p.h
            && by + h > p.y
    });
    Ok(!overlap)
}
