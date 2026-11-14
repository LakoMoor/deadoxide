//! The item subclasses of package `zombie.inventory.types`: each holds what its `load` reads after
//! `InventoryItem.load`.
pub mod alarm_clock;
pub mod clothing;
pub mod food;
pub mod hand_weapon;
pub mod literature;
pub mod moveable;

use crate::java_nio::{read_string, ByteBuffer};
use crate::zombie::inventory::inventory_item::ItemFactory;
use crate::zombie::inventory::item_container::ItemContainer;
use crate::zombie::radio::devices::device_data::DeviceData;
use crate::zombie::world_map::symbols::world_map_symbols::WorldMapSymbols;

/// zombie.inventory.types.Key, the saved fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeySave {
    /// `None` where the save holds -1: `Key.setKeyId(-1)` draws a new id from `Rand` (not modelled).
    pub key_id: Option<i32>,
    pub number_of_key: i8,
}

impl KeySave {
    /// zombie.inventory.types.Key.load after `super.load`.
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let key_id = b.get_int()?;
        Ok(KeySave {
            key_id: (key_id != -1).then_some(key_id),
            number_of_key: b.get()?,
        })
    }
}

/// zombie.inventory.types.InventoryContainer (a bag), the saved fields.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryContainerSave {
    /// `setWeightReduction`: clamped to [0, 100].
    pub weight_reduction: i32,
    pub container: ItemContainer,
}

impl InventoryContainerSave {
    /// zombie.inventory.types.InventoryContainer.load after `super.load`: the inner container's id, the weight
    /// reduction, then `ItemContainer.load` into a cleared container.
    pub fn load(
        b: &mut ByteBuffer,
        world_version: i32,
        factory: &dyn ItemFactory,
    ) -> Result<Self, String> {
        let con = b.get_int()?;
        let weight_reduction = b.get_int()?.clamp(0, 100);
        let mut container = ItemContainer {
            id: con,
            ..ItemContainer::default()
        };
        container.load(b, world_version, factory)?;
        Ok(InventoryContainerSave {
            weight_reduction,
            container,
        })
    }
}

/// zombie.inventory.types.Radio, the part after Moveable.load: the device state where saved.
#[derive(Clone, Debug, PartialEq)]
pub struct RadioSave {
    pub device_data: Option<DeviceData>,
}

impl RadioSave {
    /// zombie.inventory.types.Radio.load after `super.load` (Moveable.load).
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let device_data = if b.get()? != 0 {
            Some(DeviceData::load(b)?)
        } else {
            None
        };
        Ok(RadioSave { device_data })
    }
}

/// zombie.inventory.types.MapItem, the saved fields.
#[derive(Clone, Debug, PartialEq)]
pub struct MapItemSave {
    pub map_id: String,
    pub symbols: WorldMapSymbols,
}

impl MapItemSave {
    /// zombie.inventory.types.MapItem.load after `super.load`.
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        Ok(MapItemSave {
            map_id: read_string(b)?,
            symbols: WorldMapSymbols::load(b)?,
        })
    }
}
