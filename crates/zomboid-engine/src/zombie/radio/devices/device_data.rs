//! zombie.radio.devices.DeviceData: the radio/television state of a device (world object or item), as a save stores it.
use crate::java_nio::{read_string, ByteBuffer};

/// zombie.radio.devices.DevicePresets, the saved fields: (name, frequency) entries up to `max_presets`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DevicePresets {
    pub max_presets: i32,
    pub presets: Vec<(String, i32)>,
}

impl DevicePresets {
    /// zombie.radio.devices.DevicePresets.load: entries past `maxPresets` are read and dropped.
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let mut p = DevicePresets {
            max_presets: b.get_int()?,
            presets: Vec::new(),
        };
        let entries = b.get_int()?;
        for _ in 0..entries {
            let name = read_string(b)?;
            let freq = b.get_int()?;
            if (p.presets.len() as i64) < i64::from(p.max_presets) {
                p.presets.push((name, freq));
            }
        }
        Ok(p)
    }
}

/// zombie.radio.devices.DeviceData, the saved fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeviceData {
    pub device_name: String,
    pub two_way: bool,
    pub transmit_range: i32,
    pub mic_range: i32,
    pub mic_is_muted: bool,
    pub base_volume_range: f32,
    pub device_volume: f32,
    pub is_portable: bool,
    pub is_television: bool,
    pub is_high_tier: bool,
    pub is_turned_on: bool,
    pub channel: i32,
    pub min_channel_range: i32,
    pub max_channel_range: i32,
    pub is_battery_powered: bool,
    pub has_battery: bool,
    pub power_delta: f32,
    pub use_delta: f32,
    pub headphone_type: i32,
    /// `None` where the save has no presets (the object keeps a fresh `DevicePresets`).
    pub presets: Option<DevicePresets>,
    pub media_index: i16,
    pub media_type: i8,
    pub media_item: Option<String>,
    pub no_transmit: bool,
}

impl DeviceData {
    /// zombie.radio.devices.DeviceData.load (ByteBuffer, int, boolean)
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let mut d = DeviceData {
            device_name: read_string(b)?,
            ..DeviceData::default()
        };
        d.two_way = b.get()? != 0;
        d.transmit_range = b.get_int()?;
        d.mic_range = b.get_int()?;
        d.mic_is_muted = b.get()? != 0;
        d.base_volume_range = b.get_float()?;
        d.device_volume = b.get_float()?;
        d.is_portable = b.get()? != 0;
        d.is_television = b.get()? != 0;
        d.is_high_tier = b.get()? != 0;
        d.is_turned_on = b.get()? != 0;
        d.channel = b.get_int()?;
        d.min_channel_range = b.get_int()?;
        d.max_channel_range = b.get_int()?;
        d.is_battery_powered = b.get()? != 0;
        d.has_battery = b.get()? != 0;
        d.power_delta = b.get_float()?;
        d.use_delta = b.get_float()?;
        d.headphone_type = b.get_int()?;
        if b.get()? != 0 {
            d.presets = Some(DevicePresets::load(b)?);
        }
        d.media_index = b.get_short()?;
        d.media_type = b.get()?;
        if b.get()? != 0 {
            d.media_item = Some(read_string(b)?);
        }
        d.no_transmit = b.get()? != 0;
        Ok(d)
    }
}
