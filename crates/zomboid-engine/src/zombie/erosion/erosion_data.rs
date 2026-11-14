//! zombie.erosion.ErosionData: the erosion state a chunk save carries per chunk and per square, with the category
//! data of zombie.erosion.categories.* (`ErosionRegions` decides which category a (region, category) pair is).
use crate::java_nio::{unpack_byte_to_float_unit, ByteBuffer};

/// zombie.erosion.ErosionData.Chunk
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chunk {
    pub init: bool,
    pub e_tick_stamp: i32,
    pub epoch: i32,
    pub moisture: f32,
    pub minerals: f32,
    pub soil: i32,
}

impl Chunk {
    /// zombie.erosion.ErosionData.Chunk.load
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let mut c = Chunk {
            e_tick_stamp: -1,
            epoch: -1,
            ..Chunk::default()
        };
        c.init = b.get()? != 0;
        if c.init {
            c.e_tick_stamp = b.get_int()?;
            c.epoch = b.get_int()?;
            c.moisture = b.get_float()?;
            c.minerals = b.get_float()?;
            c.soil = i32::from(b.get()?);
        }
        Ok(c)
    }
}

/// The category class `ErosionRegions.getCategory(regionID, categoryID)` returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    NatureTrees,
    NatureBush,
    NaturePlants,
    NatureGeneric,
    StreetCracks,
    WallVines,
    WallCracks,
    Flowerbed,
}

/// zombie.erosion.ErosionRegions static init: region 0 (blends_natural_01) trees, bushes, plants, generic; region 1
/// street cracks; region 2 wall vines, wall cracks; region 3 flower beds.
pub fn get_category(region_id: i32, category_id: i32) -> Option<Category> {
    use Category::*;
    Some(match (region_id, category_id) {
        (0, 0) => NatureTrees,
        (0, 1) => NatureBush,
        (0, 2) => NaturePlants,
        (0, 3) => NatureGeneric,
        (1, 0) => StreetCracks,
        (2, 0) => WallVines,
        (2, 1) => WallCracks,
        (3, 0) => Flowerbed,
        _ => return None,
    })
}

/// The "hasTop" part of WallVines.CategoryData / WallCracks.CategoryData.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Top {
    pub game_obj: i32,
    pub spawn_time: i32,
    pub cur_id: i32,
    pub alpha: f32,
}

/// zombie.erosion.categories.ErosionCategory.Data and the subclass fields (unused ones stay at their defaults).
#[derive(Clone, Debug, PartialEq)]
pub struct CategoryData {
    pub category: Category,
    pub region_id: i32,
    pub category_id: i32,
    pub do_nothing: bool,
    pub has_spawned: bool,
    pub stage: i32,
    pub disp_season: i32,
    pub disp_bloom: bool,
    pub game_obj: i32,
    pub max_stage: i32,
    pub spawn_time: i32,
    pub not_grass: bool,
    pub cur_id: i32,
    pub has_grass: bool,
    pub alpha: f32,
    pub has_top: Option<Top>,
}

/// zombie.erosion.categories.ErosionCategory.loadCategoryData: region and category bytes, then the subclass load.
pub fn load_category_data(b: &mut ByteBuffer) -> Result<CategoryData, String> {
    let region_id = i32::from(b.get()?);
    let category_id = i32::from(b.get()?);
    let category = get_category(region_id, category_id).ok_or_else(|| {
        format!("java.lang.IndexOutOfBoundsException: erosion category {region_id}/{category_id}")
    })?;
    // ErosionCategory.Data.load
    let disp_season = i32::from(b.get()?);
    let flags = i32::from(b.get()?);
    let stage = if flags & 8 != 0 {
        1
    } else if flags & 16 != 0 {
        2
    } else if flags & 32 != 0 {
        3
    } else if flags & 64 != 0 {
        4
    } else if flags & 128 != 0 {
        i32::from(b.get()?)
    } else {
        0
    };
    let mut d = CategoryData {
        category,
        region_id,
        category_id,
        do_nothing: flags & 1 != 0,
        has_spawned: flags & 2 != 0,
        stage,
        disp_season,
        disp_bloom: flags & 4 != 0,
        game_obj: 0,
        max_stage: 0,
        spawn_time: 0,
        not_grass: false,
        cur_id: 0,
        has_grass: false,
        alpha: 0.0,
        has_top: None,
    };
    use Category::*;
    d.game_obj = i32::from(b.get()?);
    match category {
        Flowerbed => {}
        NatureBush | NatureTrees => {
            d.max_stage = i32::from(b.get()?);
            d.spawn_time = i32::from(b.get_short()?);
        }
        NatureGeneric => {
            d.max_stage = i32::from(b.get()?);
            d.spawn_time = i32::from(b.get_short()?);
            d.not_grass = b.get()? != 0;
        }
        NaturePlants => d.spawn_time = i32::from(b.get_short()?),
        StreetCracks => {
            d.max_stage = i32::from(b.get()?);
            d.spawn_time = i32::from(b.get_short()?);
            d.cur_id = b.get_int()?;
            d.has_grass = b.get()? != 0;
        }
        WallCracks => {
            d.spawn_time = i32::from(b.get_short()?);
            d.cur_id = b.get_int()?;
            d.alpha = b.get_float()?;
            if b.get()? != 0 {
                d.has_top = Some(Top {
                    game_obj: i32::from(b.get()?),
                    spawn_time: i32::from(b.get_short()?),
                    cur_id: b.get_int()?,
                    alpha: b.get_float()?,
                });
            }
        }
        WallVines => {
            d.max_stage = i32::from(b.get()?);
            d.spawn_time = i32::from(b.get_short()?);
            d.cur_id = b.get_int()?;
            if b.get()? != 0 {
                d.has_top = Some(Top {
                    game_obj: i32::from(b.get()?),
                    spawn_time: i32::from(b.get_short()?),
                    cur_id: b.get_int()?,
                    alpha: 0.0,
                });
            }
        }
    }
    Ok(d)
}

/// zombie.erosion.ErosionData.Square, the saved part.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Square {
    pub init: bool,
    pub do_nothing: bool,
    pub noise_main_byte: i8,
    pub noise_main: f32,
    pub noise_main_int: i32,
    pub soil: i32,
    pub magic_num_byte: i8,
    pub magic_num: f32,
    pub regions: Vec<CategoryData>,
}

impl Square {
    /// zombie.erosion.ErosionData.Square.load
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let flags = i32::from(b.get()?);
        let mut s = Square {
            init: flags & 1 != 0,
            ..Square::default()
        };
        if s.init {
            s.do_nothing = flags & 2 != 0;
            s.noise_main_byte = b.get()?;
            s.noise_main = unpack_byte_to_float_unit(s.noise_main_byte);
            s.noise_main_int = (s.noise_main * 100.0).floor() as i32;
            s.soil = i32::from(b.get()?);
            s.magic_num_byte = b.get()?;
            s.magic_num = unpack_byte_to_float_unit(s.magic_num_byte);
            let count = if flags & 4 != 0 {
                1
            } else if flags & 8 != 0 {
                2
            } else if flags & 16 != 0 {
                3
            } else if flags & 32 != 0 {
                4
            } else if flags & 64 != 0 {
                i32::from(b.get()?)
            } else {
                0
            };
            for _ in 0..count {
                s.regions.push(load_category_data(b)?);
            }
        }
        Ok(s)
    }
}
