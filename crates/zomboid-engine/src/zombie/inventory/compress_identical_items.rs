//! zombie.inventory.CompressIdenticalItems: identical items are saved once with a repeat count and the other ids.
use super::inventory_item::{InventoryItem, ItemFactory};
use crate::java_nio::ByteBuffer;

/// zombie.inventory.CompressIdenticalItems.load: the items in save order (`None` for the ones the original could not
/// create; it keeps a null in `savedItems` for each, which the worn/attached item indexes count).
pub fn load(
    b: &mut ByteBuffer,
    world_version: i32,
    factory: &dyn ItemFactory,
) -> Result<Vec<Option<InventoryItem>>, String> {
    let mut saved: Vec<Option<InventoryItem>> = Vec::new();
    let count = b.get_short()?;
    for _ in 0..count {
        let identical = b.get_int()?;
        let item_start = b.position();
        match InventoryItem::load_item(b, world_version, factory)? {
            None => {
                let id_list_bytes = if identical > 1 {
                    (identical - 1) as usize * 4
                } else {
                    0
                };
                b.set_position(b.position() + id_list_bytes)?;
                for _ in 0..identical {
                    saved.push(None);
                }
            }
            Some(first) => {
                let mut item = Some(first);
                for i in 0..identical {
                    if i > 0 {
                        b.set_position(item_start)?;
                        item = InventoryItem::load_item(b, world_version, factory)?;
                    }
                    saved.push(item.clone());
                }
                for i in 1..identical {
                    let id = b.get_int()?;
                    let n = saved.len() - identical as usize + i as usize;
                    if let Some(it) = saved[n].as_mut() {
                        it.id = id;
                    }
                }
            }
        }
    }
    Ok(saved)
}
