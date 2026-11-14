//! zombie.inventory.ItemContainer: a container of an object, corpse or bag, as a save stores it.
use super::compress_identical_items;
use super::inventory_item::{InventoryItem, ItemFactory};
use crate::java_nio::{read_string, ByteBuffer};

/// zombie.inventory.ItemContainer, the loaded part.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemContainer {
    pub id: i32,
    pub r#type: String,
    pub explored: bool,
    /// zombie.inventory.ItemContainer.items: the items that could be created, in save order.
    pub items: Vec<InventoryItem>,
    pub has_been_looted: bool,
    pub capacity: i32,
}

impl ItemContainer {
    /// zombie.inventory.ItemContainer.load: also returns `savedItems` (with the uncreatable ones as `None`), which
    /// worn and attached item indexes refer to.
    pub fn load(
        &mut self,
        b: &mut ByteBuffer,
        factory: &dyn ItemFactory,
    ) -> Result<Vec<Option<InventoryItem>>, String> {
        self.r#type = read_string(b)?;
        self.explored = b.get()? != 0;
        let saved = compress_identical_items::load(b, factory)?;
        self.items = saved.iter().flatten().cloned().collect();
        self.has_been_looted = b.get()? != 0;
        self.capacity = b.get_int()?;
        Ok(saved)
    }
}
