//! zombie.iso.IsoMetaGrid: so far the lotheader loader (`MetaGridLoaderThread.loadCell`).
use super::building_def::BuildingDef;
use super::building_id;
use super::iso_lot::{self, RandomAccess};
use super::iso_meta_cell::IsoMetaCell;
use super::lot_header::{LotHeader, LOTHEADER_MAGIC};
use super::map_files::MapFiles;
use super::meta_object::MetaObject;
use super::room_def::{RoomDef, RoomRect};
use super::room_id;
use crate::java;
use crate::zombie::core::math::pz_math;

/// zombie.iso.IsoMetaGrid.higherPriority300x300CellExists
pub fn higher_priority_300x300_cell_exists(
    map_files: &[MapFiles],
    priority: usize,
    cell300_x: i32,
    cell300_y: i32,
) -> bool {
    map_files[..priority]
        .iter()
        .any(|m| m.has_cell300(cell300_x, cell300_y))
}

/// zombie.iso.IsoMetaGrid.MetaGridLoaderThread.loadCell (MapFiles, int, int): reads one lotheader into `info`
/// (tile names, size, levels, zombie intensity) and appends its rooms and buildings to `meta_cell`.
///
/// `all_map_files[priority]` is the folder `info` belongs to. `rand_next` is `Rand.Next(int)` (one draw of 4 per room,
/// deciding `lightsActive`). `voronoi_values` are the `ZombieVoronoi.evaluateCellCutoff` tables multiplied into the
/// intensities; pass none until world generation is ported. `Err` is the exception the original logs before moving on:
/// whatever was stored before it stays.
#[allow(clippy::too_many_arguments)] // the original reads these from the thread, the grid and globals
pub fn load_cell(
    all_map_files: &[MapFiles],
    priority: usize,
    info: &mut LotHeader,
    meta_cell: &mut IsoMetaCell,
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
        let s =
            iso_lot::read_string(&mut input).ok_or("java.lang.NullPointerException: tile name")?;
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
        let name =
            iso_lot::read_string(&mut input).ok_or("java.lang.NullPointerException: room name")?;
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
    let base = meta_cell.room_list.len();
    for mut room_def in room_list {
        let index = meta_cell.room_list.len();
        room_def.id = room_id::make_id(w_x, w_y, index as i32);
        room_def.meta_id = room_def.calculate_meta_id(w_x, w_y);
        meta_cell.rooms.insert(room_def.id, index);
        meta_cell.room_by_meta_id.insert(room_def.meta_id, index);
        meta_cell.room_list.push(room_def);
    }
    let file_rooms = meta_cell.room_list.len() - base;

    let num_buildings = iso_lot::read_int(&mut input)?;
    for n in 0..num_buildings {
        let mut building_def = BuildingDef::default();
        let numb_rooms = iso_lot::read_int(&mut input)?;
        building_def.id = building_id::make_id(w_x, w_y, n);
        for _ in 0..numb_rooms {
            let room_index = iso_lot::read_int(&mut input)?;
            if room_index < 0 || room_index as usize >= file_rooms {
                return Err(format!("java.lang.IndexOutOfBoundsException: Index {room_index} out of bounds for length {file_rooms}"));
            }
            let r = base + room_index as usize;
            meta_cell.room_list[r].building = Some(building_def.id);
            if meta_cell.room_list[r].is_empty_outside() {
                building_def.emptyoutside.push(r);
            } else {
                building_def.rooms.push(r);
            }
        }
        building_def.calculate_bounds(&meta_cell.room_list);
        let cell300_x = building_def.x / 300;
        let cell300_y = building_def.y / 300;
        if !higher_priority_300x300_cell_exists(all_map_files, priority, cell300_x, cell300_y) {
            building_def.meta_id = building_def.calculate_meta_id(&meta_cell.room_list, w_x, w_y);
            meta_cell
                .building_by_meta_id
                .insert(building_def.meta_id, meta_cell.buildings.len());
            meta_cell.buildings.push(building_def);
        }
    }

    let zombie_intensity = input.read_bytes(1024);
    if zombie_intensity.len() != 1024 {
        return Err(format!(
            "java.io.EOFException: wx={w_x}, wy={w_y}, nBytes={}, this.zombieIntensity.length=1024",
            zombie_intensity.len()
        ));
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
            // The original also pushes the value into an already created IsoMetaChunk; chunks are not ported yet.
        }
    }
    Ok(())
}
