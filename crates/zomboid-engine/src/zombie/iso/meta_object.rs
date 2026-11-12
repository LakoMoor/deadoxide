/// zombie.iso.MetaObject: an object placed in a room by the map editor; coordinates are relative to the room's corner.
#[derive(Clone, Debug, PartialEq)]
pub struct MetaObject {
    pub r#type: i32,
    pub x: i32,
    pub y: i32,
    pub used: bool,
}
impl MetaObject {
    pub fn new(r#type: i32, x: i32, y: i32) -> Self {
        MetaObject {
            r#type,
            x,
            y,
            used: false,
        }
    }
}
