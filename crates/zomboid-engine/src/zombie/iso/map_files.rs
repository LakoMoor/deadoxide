use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::lot_header::LotHeader;
use crate::java;
use crate::zombie::core::utils::boolean_grid::BooleanGrid;

/// zombie.iso.MapFiles: the cell files of one `media/maps/<name>` folder. Lower `priority` wins where folders overlap.
#[derive(Debug)]
pub struct MapFiles {
    pub map_directory_name: String,
    pub map_directory_absolute_path: PathBuf,
    pub priority: usize,
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub info_headers: HashMap<String, LotHeader>,
    pub info_header_names: Vec<String>,
    pub info_file_names: HashMap<String, PathBuf>,
    pub bg_has_cell: BooleanGrid,
    pub min_cell300_x: i32,
    pub min_cell300_y: i32,
    pub max_cell300_x: i32,
    pub max_cell300_y: i32,
    pub bg_has_cell300: BooleanGrid,
}

fn floor(v: f32) -> i32 {
    (v as f64).floor() as i32
}

impl MapFiles {
    pub fn new(
        map_directory_name: &str,
        map_directory_absolute_path: &Path,
        priority: usize,
    ) -> Self {
        MapFiles {
            map_directory_name: map_directory_name.to_string(),
            map_directory_absolute_path: map_directory_absolute_path.to_path_buf(),
            priority,
            min_x: i32::MAX,
            min_y: i32::MAX,
            max_x: i32::MIN,
            max_y: i32::MIN,
            info_headers: HashMap::new(),
            info_header_names: Vec::new(),
            info_file_names: HashMap::new(),
            bg_has_cell: BooleanGrid::new(0, 0),
            min_cell300_x: 0,
            min_cell300_y: 0,
            max_cell300_x: 0,
            max_cell300_y: 0,
            bg_has_cell300: BooleanGrid::new(0, 0),
        }
    }
    /// zombie.iso.MapFiles.getWidthInCells
    pub fn get_width_in_cells(&self) -> i32 {
        self.max_x - self.min_x + 1
    }
    /// zombie.iso.MapFiles.getHeightInCells
    pub fn get_height_in_cells(&self) -> i32 {
        self.max_y - self.min_y + 1
    }

    /// zombie.iso.MapFiles.createLotHeader: cell coordinates come from the file name `<x>_<y>.lotheader`.
    fn create_lot_header(&mut self, fixed2x: bool, file_name: &str) -> Result<LotHeader, String> {
        let mut split = file_name.split('_');
        let nfe = |s: &str| {
            format!("java.lang.NumberFormatException: For input string: \"{s}\" ({file_name})")
        };
        let a = java::trim(split.next().unwrap_or(""));
        let b = split
            .next()
            .ok_or("java.lang.ArrayIndexOutOfBoundsException")?
            .replace(".lotheader", "");
        let b = java::trim(&b);
        let x = java::parse_int(a).ok_or_else(|| nfe(a))?;
        let y = java::parse_int(b).ok_or_else(|| nfe(b))?;
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
        let mut lot_header = LotHeader::new(x, y);
        lot_header.fixed2x = fixed2x;
        lot_header.file_name = file_name.to_string();
        lot_header.absolute_file_path = self.map_directory_absolute_path.join(file_name);
        Ok(lot_header)
    }

    /// zombie.iso.MapFiles.load: indexes the folder. `fixed2x` is `ChooseGameInfo.getMapDetails(name).isFixed2x()`.
    /// The directory is listed in sorted order here (Java's `File.list` order is unspecified).
    pub fn load(&mut self, fixed2x: bool) -> Result<bool, String> {
        if !self.map_directory_absolute_path.is_dir() {
            return Ok(false);
        }
        let mut names: Vec<String> = Vec::new();
        for e in std::fs::read_dir(&self.map_directory_absolute_path).map_err(|e| e.to_string())? {
            names.push(
                e.map_err(|e| e.to_string())?
                    .file_name()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        names.sort_unstable();
        for file_name in names {
            if file_name.ends_with(".lotheader") {
                let info = self.create_lot_header(fixed2x, &file_name)?;
                self.info_file_names
                    .insert(file_name.clone(), info.absolute_file_path.clone());
                self.info_headers.insert(file_name.clone(), info);
                self.info_header_names.push(file_name);
            } else if file_name.ends_with(".lotpack") || file_name.starts_with("chunkdata_") {
                self.info_file_names.insert(
                    file_name.clone(),
                    self.map_directory_absolute_path.join(&file_name),
                );
            }
        }
        Ok(true)
    }

    /// zombie.iso.MapFiles.postLoad: which 256-cells exist, and which legacy 300-cells are fully covered by them.
    pub fn post_load(&mut self) {
        if self.min_x > self.max_x {
            self.min_x = 0;
            self.max_x = 0;
            self.min_y = 0;
            self.max_y = 0;
        }
        self.bg_has_cell = BooleanGrid::new(self.get_width_in_cells(), self.get_height_in_cells());
        for y in 0..self.get_height_in_cells() {
            for x in 0..self.get_width_in_cells() {
                let has = self.info_headers.contains_key(&format!(
                    "{}_{}.lotheader",
                    self.min_x + x,
                    self.min_y + y
                ));
                self.bg_has_cell.set_value(x, y, has);
            }
        }
        self.min_cell300_x = floor(self.min_x as f32 * 256.0 / 300.0);
        self.min_cell300_y = floor(self.min_y as f32 * 256.0 / 300.0);
        self.max_cell300_x = floor((self.max_x + 1) as f32 * 256.0 / 300.0);
        self.max_cell300_y = floor((self.max_y + 1) as f32 * 256.0 / 300.0);
        self.bg_has_cell300 = BooleanGrid::new(
            self.max_cell300_x - self.min_cell300_x + 1,
            self.max_cell300_y - self.min_cell300_y + 1,
        );
        for cell300_y in self.min_cell300_y..=self.max_cell300_y {
            for cell300_x in self.min_cell300_x..=self.max_cell300_x {
                let cell256_x = floor(cell300_x as f32 * 300.0 / 256.0);
                let cell256_y = floor(cell300_y as f32 * 300.0 / 256.0);
                if self.has_cell(cell256_x, cell256_y)
                    && self.has_cell(cell256_x + 1, cell256_y + 1)
                {
                    self.bg_has_cell300.set_value(
                        cell300_x - self.min_cell300_x,
                        cell300_y - self.min_cell300_y,
                        true,
                    );
                }
            }
        }
    }

    /// zombie.iso.MapFiles.isValidCellPos
    pub fn is_valid_cell_pos(&self, cell_x: i32, cell_y: i32) -> bool {
        cell_x >= self.min_x && cell_y >= self.min_y && cell_x <= self.max_x && cell_y <= self.max_y
    }
    /// zombie.iso.MapFiles.getLotHeader
    pub fn get_lot_header(&self, cell_x: i32, cell_y: i32) -> Option<&LotHeader> {
        if self.has_cell(cell_x, cell_y) {
            self.info_headers
                .get(&format!("{cell_x}_{cell_y}.lotheader"))
        } else {
            None
        }
    }
    /// zombie.iso.MapFiles.hasCell
    pub fn has_cell(&self, cell_x: i32, cell_y: i32) -> bool {
        self.is_valid_cell_pos(cell_x, cell_y)
            && self
                .bg_has_cell
                .get_value(cell_x - self.min_x, cell_y - self.min_y)
    }
    /// zombie.iso.MapFiles.hasCell300
    pub fn has_cell300(&self, cell300_x: i32, cell300_y: i32) -> bool {
        self.bg_has_cell300.get_value(
            cell300_x - self.min_cell300_x,
            cell300_y - self.min_cell300_y,
        )
    }
}
