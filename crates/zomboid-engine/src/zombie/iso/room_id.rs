//! zombie.iso.RoomID: cell coordinates and a per-cell index packed into a long.

/// zombie.iso.RoomID.makeID
pub fn make_id(cell_x: i32, cell_y: i32, room_index: i32) -> i64 {
    let hi = cell_y << 16 | cell_x;
    (hi as i64) << 32 | room_index as i64
}
/// zombie.iso.RoomID.getCellX
pub fn get_cell_x(id: i64) -> i32 {
    (id >> 32) as i32 & 65535
}
/// zombie.iso.RoomID.getCellY
pub fn get_cell_y(id: i64) -> i32 {
    ((id >> 32) as i32) >> 16 & 65535
}
/// zombie.iso.RoomID.getIndex
pub fn get_index(id: i64) -> i32 {
    (id & 4294967295) as i32
}
/// zombie.iso.RoomID.isSameCell
pub fn is_same_cell(id: i64, cell_x: i32, cell_y: i32) -> bool {
    get_cell_x(id) == cell_x && get_cell_y(id) == cell_y
}
