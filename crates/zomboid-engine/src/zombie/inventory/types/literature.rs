//! zombie.inventory.types.Literature: pages, writing and learned recipes as a save stores them.
use crate::java_nio::{read_string, BitHeaderRead, ByteBuffer, HeaderSize};

/// zombie.inventory.types.Literature, the fields its load resets and reads.
#[derive(Clone, Debug, PartialEq)]
pub struct LiteratureSave {
    pub number_of_pages: i32,
    pub already_read_pages: i32,
    pub can_be_write: bool,
    /// `customPages`: page 1, 2, ... in order.
    pub custom_pages: Option<Vec<String>>,
    pub locked_by: Option<String>,
    pub learned_recipes: Option<Vec<String>>,
}

impl LiteratureSave {
    /// zombie.inventory.types.Literature.load after `super.load`: the page counts are stored as a byte, short or int
    /// as flags 2 and 4 say.
    pub fn load(b: &mut ByteBuffer, world_version: i32) -> Result<Self, String> {
        let mut l = LiteratureSave {
            number_of_pages: -1,
            already_read_pages: 0,
            can_be_write: false,
            custom_pages: None,
            locked_by: None,
            learned_recipes: None,
        };
        let size = if world_version >= 226 {
            HeaderSize::Short
        } else {
            HeaderSize::Byte
        };
        let bits = BitHeaderRead::alloc_read(size, b)?;
        if !bits.equals(0) {
            let mut number_page_type = 0;
            if bits.has_flags(1)? {
                if bits.has_flags(2)? {
                    number_page_type = 1;
                    l.number_of_pages = i32::from(b.get_short()?);
                } else if bits.has_flags(4)? {
                    number_page_type = 2;
                    l.number_of_pages = b.get_int()?;
                } else {
                    l.number_of_pages = i32::from(b.get()?);
                }
            }
            if bits.has_flags(8)? {
                l.already_read_pages = match number_page_type {
                    1 => i32::from(b.get_short()?),
                    2 => b.get_int()?,
                    _ => i32::from(b.get()?),
                };
            }
            l.can_be_write = bits.has_flags(16)?;
            if bits.has_flags(32)? {
                let size = b.get_int()?;
                if size > 0 {
                    let mut pages = Vec::new();
                    for _ in 0..size {
                        pages.push(read_string(b)?);
                    }
                    l.custom_pages = Some(pages);
                }
            }
            if bits.has_flags(64)? {
                l.locked_by = Some(read_string(b)?);
            }
            if world_version >= 226 && bits.has_flags(128)? {
                let num_recipes = b.get_short()?;
                let mut recipes = Vec::new();
                for _ in 0..num_recipes {
                    recipes.push(read_string(b)?);
                }
                l.learned_recipes = Some(recipes);
            }
        }
        Ok(l)
    }
}
