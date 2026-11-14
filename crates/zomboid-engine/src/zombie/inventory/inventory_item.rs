//! zombie.inventory.InventoryItem: an item as a save stores it. So far the frame of `loadItem` (length, registry id,
//! save type) and the item id; the rest of the item's bytes are kept raw until the per-type loads are ported.
use crate::java_nio::ByteBuffer;

/// zombie.inventory.InventoryItem, the loaded part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryItem {
    pub registry_id: i16,
    /// `getFullType()`: the WorldDictionary entry of `registry_id`.
    pub full_type: String,
    /// The byte `save()` writes after the registry id (the item class).
    pub save_type: i8,
    /// zombie.inventory.InventoryItem.id
    pub id: i32,
    /// The item's bytes after the registry id and save type (what `InventoryItem.load` and the subclass loads read).
    pub data: Vec<u8>,
}

/// What item loading needs from the running game: the dictionary lookup that `InventoryItemFactory.CreateItem(short)`
/// does (`None` where it returns null: unknown id, invalid entry or no such script).
pub trait ItemFactory {
    fn create_item(&self, registry_id: i16) -> Option<String>;
    /// `ScriptManager.instance.FindItem(fullType)` exists and `getObsolete()`.
    fn is_obsolete(&self, full_type: &str) -> bool;
}

impl InventoryItem {
    /// zombie.inventory.InventoryItem.loadItem (ByteBuffer, int): an int length, then the item. `Ok(None)` where the
    /// original skips an item it cannot create (the length tells it how far).
    pub fn load_item(
        b: &mut ByteBuffer,
        factory: &dyn ItemFactory,
    ) -> Result<Option<Self>, String> {
        let data_len = b.get_int()?;
        if data_len <= 0 {
            return Err(format!("java.io.IOException: InventoryItem.loadItem() invalid item data length: {data_len}"));
        }
        let position = b.position();
        let end = position + data_len as usize;
        let registry_id = b.get_short()?;
        let save_type = b.get()?;
        let Some(full_type) = factory.create_item(registry_id) else {
            // item == null: skip to the end of its bytes.
            b.set_position(end.max(b.position()))?;
            return Ok(None);
        };
        // InventoryItem.load starts with the id; every subclass load calls it first.
        let data = b.get_bytes(end.saturating_sub(b.position()))?.to_vec();
        let id = data
            .get(..4)
            .map_or(0, |d| i32::from_be_bytes([d[0], d[1], d[2], d[3]]));
        Ok(Some(InventoryItem {
            registry_id,
            full_type,
            save_type,
            id,
            data,
        }))
    }
}
