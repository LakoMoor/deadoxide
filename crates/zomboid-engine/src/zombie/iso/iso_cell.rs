//! zombie.iso.IsoCell: so far placing a chunk's tile data as squares and objects.
use super::cell_loader::{self, LoadContext};
use super::iso_chunk::IsoChunk;
use super::iso_grid_square::IsoGridSquare;
use super::iso_lot::IsoLot;
use super::lot_header::LotHeader;
use crate::java::JavaRandom;

/// zombie.iso.IsoCell.PlaceLot (IsoLot, int, int, int, IsoChunk, int, int, boolean[]): creates the squares and objects
/// of one chunk from its tile data. `b_done_squares` marks columns already filled by a higher-priority map folder.
/// Returns the number of non-empty columns. On an exception the original logs it and blanks the area; so does this,
/// returning the error text as well.
#[allow(clippy::too_many_arguments)]
pub fn place_lot<R: JavaRandom>(
    ctx: &mut LoadContext<R>,
    lot: &IsoLot,
    info: &LotHeader,
    sx: i32,
    sy: i32,
    sz: i32,
    ch: &mut IsoChunk,
    wx: i32,
    wy: i32,
    b_done_squares: &mut [bool; 64],
) -> (i32, Option<String>) {
    let wx = wx * 8;
    let wy = wy * 8;
    let min_level = sz.max(-32);
    let max_level = (sz + lot.max_level - lot.min_level - 1).min(31);
    let mut non_empty_square_count = 0;
    let r = place_lot_inner(
        ctx,
        lot,
        info,
        sx,
        sy,
        ch,
        wx,
        wy,
        min_level,
        max_level,
        b_done_squares,
        &mut non_empty_square_count,
    );
    if r.is_err() {
        for x in wx + sx..wx + sx + 8 {
            for yx in wy + sy..wy + sy + 8 {
                for zx in min_level..=max_level {
                    // The original indexes with `z - sz` here, as written.
                    let (lx, ly) = (x - wx - sx, yx - wy - sy);
                    if (0..8).contains(&lx) && (0..8).contains(&ly) {
                        ch.set_square(lx, ly, zx - sz, None);
                    }
                }
            }
        }
    }
    (non_empty_square_count, r.err())
}

#[allow(clippy::too_many_arguments)]
fn place_lot_inner<R: JavaRandom>(
    ctx: &mut LoadContext<R>,
    lot: &IsoLot,
    info: &LotHeader,
    sx: i32,
    sy: i32,
    ch: &mut IsoChunk,
    wx: i32,
    wy: i32,
    min_level: i32,
    max_level: i32,
    b_done_squares: &mut [bool; 64],
    non_empty_square_count: &mut i32,
) -> Result<(), String> {
    if !info.fixed2x {
        return Err("IsoChunk.Fix2x (maps without fixed2x) is not ported".into());
    }
    for x in wx + sx..wx + sx + 8 {
        for y in wy + sy..wy + sy + 8 {
            if !(x >= wx && y >= wy && x < wx + 8 && y < wy + 8) {
                continue;
            }
            let col = (x - wx + (y - wy) * 8) as usize;
            if b_done_squares[col] {
                *non_empty_square_count += 1;
                continue;
            }
            for z in min_level..=max_level {
                let lx = x - wx - sx;
                let ly = y - wy - sy;
                let lz = z - info.min_level;
                let square_xyz = lx + ly * 8 + lz * 8 * 8;
                let offset_in_data = *usize::try_from(square_xyz)
                    .ok()
                    .and_then(|i| lot.offset_in_data.get(i))
                    .ok_or("java.lang.ArrayIndexOutOfBoundsException: offsetInData")?;
                if offset_in_data == -1 {
                    continue;
                }
                let num_ints = lot.data[offset_in_data as usize];
                if num_ints <= 0 {
                    continue;
                }
                if !b_done_squares[col] {
                    b_done_squares[col] = true;
                    *non_empty_square_count += 1;
                }
                if ch.get_grid_square(x - wx, y - wy, z).is_none() {
                    ch.set_square(x - wx, y - wy, z, Some(IsoGridSquare::new(x, y, z)));
                }
                for xx in -1..=1 {
                    for yy in -1..=1 {
                        if (xx != 0 || yy != 0)
                            && xx + x - wx >= 0
                            && xx + x - wx < 8
                            && yy + y - wy >= 0
                            && yy + y - wy < 8
                            && ch.get_grid_square(x + xx - wx, y + yy - wy, z).is_none()
                        {
                            ch.set_square(
                                x + xx - wx,
                                y + yy - wy,
                                z,
                                Some(IsoGridSquare::new(x + xx, y + yy, z)),
                            );
                        }
                    }
                }
                let room_id = ctx
                    .meta_grid
                    .get_room_at(x, y, z)
                    .map_or(-1, |r| ctx.meta_grid.room(r).id);
                // zombie.iso.IsoGridSquare.setRoomID: instantiates the runtime room (and building) on first contact.
                let room = if room_id != -1 {
                    let rand = &mut *ctx.rand;
                    ctx.meta_grid.get_room_by_id(room_id, &mut |n| rand.next(n))
                } else {
                    None
                };
                // An "emptyoutside" room over the square (a porch, a roofed yard) names the building whose roof hides it.
                let roof_hide_building = match ctx
                    .meta_grid
                    .get_empty_outside_at(x, y, z)
                    .map(|r| ctx.meta_grid.room(r).id)
                {
                    Some(id) => {
                        let rand = &mut *ctx.rand;
                        ctx.meta_grid
                            .get_room_by_id(id, &mut |n| rand.next(n))
                            .map(|r| ctx.meta_grid.iso_rooms[r].building)
                    }
                    None => None,
                };
                let square = ch.get_grid_square_mut(x - wx, y - wy, z).ok_or("square")?;
                square.room_id = room_id;
                square.room = room;
                if roof_hide_building.is_some() {
                    square.roof_hide_building = roof_hide_building;
                }

                let b_clear_objects = true;
                for n in 0..num_ints {
                    let tile_index = lot.data[(offset_in_data + 1 + n) as usize];
                    let tile = usize::try_from(tile_index)
                        .ok()
                        .and_then(|i| info.tiles_used.get(i))
                        .ok_or("java.lang.IndexOutOfBoundsException: tilesUsed")?;
                    let Some(spr) = ctx.sprites.named_map.get(tile).copied() else {
                        ctx.missing_tiles.push(tile.clone());
                        continue;
                    };
                    // `n == 0 && solidfloor && (!hidewalls || numInts > 1)` only re-sets a flag that is already true.
                    if b_clear_objects && n == 0 {
                        square.objects.clear();
                    }
                    let ty = ctx.sprites.sprite(spr).tile_type;
                    cell_loader::do_tile_object_creation(ctx, spr, ty, square, tile)?;
                }
                square.fix_stackable_objects(ctx.sprites, ctx.alias);
            }
        }
    }
    Ok(())
}
