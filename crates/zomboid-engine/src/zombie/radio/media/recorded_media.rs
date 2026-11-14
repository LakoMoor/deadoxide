//! zombie.radio.media.RecordedMedia: the save's table of recorded media (CDs, VHS tapes) that items refer to by index.
use crate::java_nio::{read_string, ByteBuffer};

/// zombie.radio.media.RecordedMedia, the part a save stores (`recorded_media.bin`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordedMedia {
    /// Media ids by index, as the save stores them. `register` (from the Lua `OnInitRecordedMedia` handler) appends
    /// ids the file does not have; that is not modelled.
    pub indexes: Vec<String>,
    pub legacy_listened_lines: Vec<String>,
    pub home_vhs_spawned: Vec<i16>,
}

impl RecordedMedia {
    /// zombie.radio.media.RecordedMedia.load: version, the ids, (version 1) the listened lines, the spawned home VHS
    /// indexes. The original logs and keeps what it read when the file is broken; this returns the error.
    pub fn load(data: &[u8]) -> Result<Self, String> {
        let mut b = ByteBuffer::wrap(data);
        let mut m = RecordedMedia::default();
        let version = b.get_int()?;
        let size = b.get_int()?;
        for _ in 0..size {
            m.indexes.push(read_string(&mut b)?);
        }
        if version == 1 {
            let size = b.get_int()?;
            for _ in 0..size {
                m.legacy_listened_lines.push(read_string(&mut b)?);
            }
        }
        let size = b.get_int()?;
        for _ in 0..size {
            m.home_vhs_spawned.push(b.get_short()?);
        }
        Ok(m)
    }

    /// zombie.radio.media.RecordedMedia.getMediaDataFromIndex: the id at `index` where `registered` says a MediaData
    /// exists for it (`getMediaData(id) != null`).
    pub fn get_media_data_from_index(
        &self,
        index: i16,
        registered: &dyn Fn(&str) -> bool,
    ) -> Option<&str> {
        let id = self.indexes.get(usize::try_from(index).ok()?)?;
        registered(id).then_some(id.as_str())
    }
}
