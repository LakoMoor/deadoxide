/// zombie.core.utils.BooleanGrid: a bit matrix, one `int` per 32 columns.
#[derive(Clone, Debug, PartialEq)]
pub struct BooleanGrid {
    width: i32,
    height: i32,
    bit_width: i32,
    value: Vec<i32>,
}

impl BooleanGrid {
    pub fn new(width: i32, height: i32) -> Self {
        let w = width / 32 + if width % 32 != 0 { 1 } else { 0 };
        BooleanGrid {
            width: w,
            height,
            bit_width: width,
            value: vec![0; (w * height).max(0) as usize],
        }
    }
    /// zombie.core.utils.BooleanGrid.clear
    pub fn clear(&mut self) {
        self.value.fill(0);
    }
    /// zombie.core.utils.BooleanGrid.fill
    pub fn fill(&mut self) {
        self.value.fill(-1);
    }
    /// zombie.core.utils.BooleanGrid.getIndex
    fn get_index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| (x + y * self.width) as usize)
    }
    /// zombie.core.utils.BooleanGrid.getValue: false outside the grid.
    pub fn get_value(&self, x: i32, y: i32) -> bool {
        if x < self.bit_width && x >= 0 && y < self.height && y >= 0 {
            self.get_index(x / 32, y)
                .is_some_and(|idx| self.value[idx] & (1 << (x & 31)) != 0)
        } else {
            false
        }
    }
    /// zombie.core.utils.BooleanGrid.setValue: ignored outside the grid.
    pub fn set_value(&mut self, x: i32, y: i32, new_value: bool) {
        if x < self.bit_width && x >= 0 && y < self.height && y >= 0 {
            if let Some(idx) = self.get_index(x / 32, y) {
                if new_value {
                    self.value[idx] |= 1 << (x & 31);
                } else {
                    self.value[idx] &= !(1 << (x & 31));
                }
            }
        }
    }
    /// zombie.core.utils.BooleanGrid.getWidth
    pub fn get_width(&self) -> i32 {
        self.bit_width
    }
    /// zombie.core.utils.BooleanGrid.getHeight
    pub fn get_height(&self) -> i32 {
        self.height
    }
}
