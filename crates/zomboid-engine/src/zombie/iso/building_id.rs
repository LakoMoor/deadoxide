//! zombie.iso.BuildingID: cell coordinates and a per-cell index packed into a long.

/// zombie.iso.BuildingID.makeID
pub fn make_id(cell_x: i32, cell_y: i32, building_index: i32) -> i64 {
    let hi = cell_x | cell_y << 16;
    (hi as i64) << 32 | building_index as i64
}
/// zombie.iso.BuildingID.getCellX
pub fn get_cell_x(id: i64) -> i32 {
    (id >> 32) as i32 & 65535
}
/// zombie.iso.BuildingID.getCellY
pub fn get_cell_y(id: i64) -> i32 {
    ((id >> 32) as i32) >> 16 & 65535
}
/// zombie.iso.BuildingID.getIndex
pub fn get_index(id: i64) -> i32 {
    (id & 4294967295) as i32
}
/// zombie.iso.BuildingID.isSameCell
pub fn is_same_cell(id: i64, cell_x: i32, cell_y: i32) -> bool {
    get_cell_x(id) == cell_x && get_cell_y(id) == cell_y
}
