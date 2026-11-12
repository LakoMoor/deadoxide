/// zombie.iso.LotHeader.LOTHEADER_MAGIC
pub const LOTHEADER_MAGIC: &[u8; 4] = b"LOTH";
/// zombie.iso.LotHeader.LOTPACK_MAGIC
pub const LOTPACK_MAGIC: &[u8; 4] = b"LOTP";

/// zombie.iso.LotHeader: what a `<x>_<y>.lotheader` says about one 256x256 cell. `map_files` is the owner's priority index.
#[derive(Clone, Debug, PartialEq)]
pub struct LotHeader {
    pub cell_x: i32,
    pub cell_y: i32,
    pub width: i32,
    pub height: i32,
    pub min_level: i32,
    pub max_level: i32,
    pub version: i32,
    pub fixed2x: bool,
    pub tiles_used: Vec<String>,
    zombie_intensity: Vec<u8>,
    pub file_name: String,
    pub absolute_file_path: std::path::PathBuf,
    /// zombie.iso.LotHeader.adjacentCells, indexed by `IsoDirections` ordinal.
    pub adjacent_cells: [bool; 8],
}

impl LotHeader {
    pub fn new(cell_x: i32, cell_y: i32) -> Self {
        LotHeader {
            cell_x,
            cell_y,
            width: 0,
            height: 0,
            min_level: 0,
            max_level: 0,
            version: 0,
            fixed2x: false,
            tiles_used: Vec::new(),
            zombie_intensity: vec![0; 1024],
            file_name: String::new(),
            absolute_file_path: Default::default(),
            adjacent_cells: [false; 8],
        }
    }
    /// zombie.iso.LotHeader.getNumLevels
    pub fn get_num_levels(&self) -> i32 {
        self.max_level - self.min_level + 1
    }
    /// zombie.iso.LotHeader.getZombieIntensity
    pub fn get_zombie_intensity(&self, i: usize) -> u8 {
        self.zombie_intensity[i]
    }
    /// zombie.iso.LotHeader.setZombieIntensity
    pub fn set_zombie_intensity(&mut self, i: usize, v: u8) {
        self.zombie_intensity[i] = v;
    }
}
