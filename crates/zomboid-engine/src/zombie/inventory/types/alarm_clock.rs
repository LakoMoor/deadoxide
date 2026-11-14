//! zombie.inventory.types.AlarmClock and AlarmClockClothing: the alarm a save stores on top of the item.
use crate::java_nio::ByteBuffer;

/// zombie.inventory.types.AlarmClock (and AlarmClockClothing), the saved alarm fields.
#[derive(Clone, Debug, PartialEq)]
pub struct AlarmClockSave {
    pub alarm_hour: i32,
    pub alarm_minutes: i32,
    pub alarm_set: bool,
    /// A double field, read as a float.
    pub ring_since: f32,
    /// Field initialiser -1; read from world version 205.
    pub force_dont_ring: i32,
}

impl AlarmClockSave {
    /// zombie.inventory.types.AlarmClock.load after `super.load` (AlarmClockClothing.load reads the same after
    /// Clothing.load).
    pub fn load(b: &mut ByteBuffer, world_version: i32) -> Result<Self, String> {
        let alarm_hour = b.get_int()?;
        let alarm_minutes = b.get_int()?;
        let alarm_set = b.get()? != 0;
        let ring_since = b.get_float()?;
        let force_dont_ring = if world_version >= 205 {
            b.get_int()?
        } else {
            -1
        };
        Ok(AlarmClockSave {
            alarm_hour,
            alarm_minutes,
            alarm_set,
            ring_since,
            force_dont_ring,
        })
    }
}
