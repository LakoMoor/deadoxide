//! Map cell containers. Layout derived from bytes of real files (build 25485521):
//! LOTP: magic "LOTP", u32 version(=1), u32 chunk_count(=1024 = 32x32), then chunk_count x u64 file offsets (LE).
//! LOTH: magic "LOTH", u32 version(=1), u32 tile_name_count, that many '\n'-terminated names, then fields not yet decoded.
use std::fmt;

#[derive(Debug)]
pub enum LotError {
    Truncated(&'static str),
    BadMagic,
    Version(u32),
    Offset { index: usize, off: u64, len: usize },
}
impl fmt::Display for LotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for LotError {}

fn u32_at(b: &[u8], p: usize, what: &'static str) -> Result<u32, LotError> {
    b.get(p..p + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
        .ok_or(LotError::Truncated(what))
}

#[derive(Debug)]
pub struct LotPack {
    pub version: u32,
    pub offsets: Vec<u64>,
}

pub fn parse_lotpack(b: &[u8]) -> Result<LotPack, LotError> {
    if b.get(..4) != Some(b"LOTP") {
        return Err(LotError::BadMagic);
    }
    let version = u32_at(b, 4, "version")?;
    if version != 1 {
        return Err(LotError::Version(version));
    }
    let n = u32_at(b, 8, "count")? as usize;
    let mut offsets = Vec::with_capacity(n);
    for i in 0..n {
        let p = 12 + i * 8;
        let s = b.get(p..p + 8).ok_or(LotError::Truncated("offset table"))?;
        let off = u64::from_le_bytes(s.try_into().unwrap());
        if off as usize > b.len() {
            return Err(LotError::Offset {
                index: i,
                off,
                len: b.len(),
            });
        }
        offsets.push(off);
    }
    Ok(LotPack { version, offsets })
}

#[derive(Debug)]
pub struct LotHeader {
    pub version: u32,
    pub tile_names: Vec<String>,
    pub rest_offset: usize,
}

pub fn parse_lotheader(b: &[u8]) -> Result<LotHeader, LotError> {
    if b.get(..4) != Some(b"LOTH") {
        return Err(LotError::BadMagic);
    }
    let version = u32_at(b, 4, "version")?;
    if version != 1 {
        return Err(LotError::Version(version));
    }
    let n = u32_at(b, 8, "tile count")? as usize;
    let mut p = 12;
    let mut tile_names = Vec::with_capacity(n);
    for _ in 0..n {
        let end = b[p..]
            .iter()
            .position(|&c| c == b'\n')
            .ok_or(LotError::Truncated("tile name"))?;
        tile_names.push(String::from_utf8_lossy(&b[p..p + end]).into_owned());
        p += end + 1;
    }
    Ok(LotHeader {
        version,
        tile_names,
        rest_offset: p,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_pack() {
        let mut b = b"LOTP".to_vec();
        b.extend(1u32.to_le_bytes());
        b.extend(1u32.to_le_bytes());
        b.extend(20u64.to_le_bytes());
        b.extend([0u8; 8]);
        assert_eq!(parse_lotpack(&b).unwrap().offsets, vec![20]);
        assert!(matches!(parse_lotpack(b"XXXX"), Err(LotError::BadMagic)));
    }
    #[test]
    fn synthetic_header() {
        let mut b = b"LOTH".to_vec();
        b.extend(1u32.to_le_bytes());
        b.extend(2u32.to_le_bytes());
        b.extend(b"a\nb\n");
        assert_eq!(parse_lotheader(&b).unwrap().tile_names, ["a", "b"]);
    }
}

// ---- header tail + chunk payload (layout from lot bytes + loader control flow; build 25485521) ----
#[derive(Debug)]
pub struct LotHeaderInfo {
    pub width: i32,
    pub height: i32,
    pub min_level: i32,
    pub max_level: i32,
}

/// Reads width,height,minLevel,maxLevel that follow the tile-name table (version 1).
pub fn parse_lotheader_dims(b: &[u8], h: &LotHeader) -> Result<LotHeaderInfo, LotError> {
    let r = |i: usize| u32_at(b, h.rest_offset + i * 4, "header dims").map(|v| v as i32);
    Ok(LotHeaderInfo {
        width: r(0)?,
        height: r(1)?,
        min_level: r(2)?,
        max_level: r(3)?,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Square {
    pub room_id: i32,
    pub tiles: Vec<i32>,
}

/// One 8x8 chunk across levels `min..=max`: index [z][x*8+y]. `None` = empty square.
#[derive(Debug)]
pub struct Chunk {
    pub levels: Vec<Vec<Option<Square>>>,
    pub bytes_read: usize,
}

pub fn parse_chunk(
    b: &[u8],
    start: usize,
    min_level: i32,
    max_level: i32,
) -> Result<Chunk, LotError> {
    let mut p = start;
    let mut rd = |what: &'static str| -> Result<i32, LotError> {
        let v = u32_at(b, p, what)? as i32;
        p += 4;
        Ok(v)
    };
    let mut skip = 0i32;
    let mut levels = Vec::new();
    for _z in min_level..=max_level {
        let mut sq: Vec<Option<Square>> = Vec::with_capacity(64);
        for _ in 0..64 {
            if skip > 0 {
                skip -= 1;
                sq.push(None);
                continue;
            }
            let n = rd("square count")?;
            if n == -1 {
                let s = rd("skip run")?;
                if s > 0 {
                    skip = s - 1;
                    sq.push(None);
                    continue;
                }
            }
            if n > 1 {
                let room_id = rd("room id")?;
                let mut tiles = Vec::with_capacity((n - 1) as usize);
                for _ in 1..n {
                    tiles.push(rd("tile")?);
                }
                sq.push(Some(Square { room_id, tiles }));
            } else {
                sq.push(None);
            }
        }
        levels.push(sq);
    }
    Ok(Chunk {
        levels,
        bytes_read: p - start,
    })
}

// ---- rooms in the header tail (layout from bytes of real files, build 25485521; NOT cross-checked with the Java loader) ----
/// One named room: a set of rectangles in cell-local square coordinates.
/// `level` is the floor (z) the room is on (observed -2..3 in cell 32_45). After the rectangles come an object count and that many
/// (type, x, y) triples; what the type numbers mean is UNKNOWN.
#[derive(Debug, Clone)]
pub struct Room {
    pub name: String,
    pub level: i32,
    pub rects: Vec<(i32, i32, i32, i32)>,
    pub objects: Vec<(i32, i32, i32)>,
}

/// Parses the room table that follows the 4 dimension ints. Returns the rooms and the offset right after them.
pub fn parse_rooms(b: &[u8], h: &LotHeader) -> Result<(Vec<Room>, usize), LotError> {
    let mut p = h.rest_offset + 16;
    let n = u32_at(b, p, "room count")? as usize;
    p += 4;
    let mut rooms = Vec::with_capacity(n);
    for _ in 0..n {
        let end = b
            .get(p..)
            .and_then(|s| s.iter().position(|&c| c == b'\n'))
            .ok_or(LotError::Truncated("room name"))?;
        let name = String::from_utf8_lossy(&b[p..p + end]).into_owned();
        p += end + 1;
        let level = u32_at(b, p, "room level")? as i32;
        let rc = u32_at(b, p + 4, "rect count")? as usize;
        p += 8;
        let mut rects = Vec::with_capacity(rc);
        for _ in 0..rc {
            let r = |i: usize| u32_at(b, p + i * 4, "rect").map(|v| v as i32);
            rects.push((r(0)?, r(1)?, r(2)?, r(3)?));
            p += 16;
        }
        let oc = u32_at(b, p, "object count")? as usize;
        p += 4;
        let mut objects = Vec::with_capacity(oc);
        for _ in 0..oc {
            let r = |i: usize| u32_at(b, p + i * 4, "room object").map(|v| v as i32);
            objects.push((r(0)?, r(1)?, r(2)?));
            p += 12;
        }
        rooms.push(Room {
            name,
            level,
            rects,
            objects,
        });
    }
    Ok((rooms, p))
}

impl Room {
    pub fn contains(&self, lx: i32, ly: i32) -> bool {
        self.rects
            .iter()
            .any(|&(x, y, w, h)| lx >= x && lx < x + w && ly >= y && ly < y + h)
    }
}

#[cfg(test)]
mod room_tests {
    use super::*;
    #[test]
    fn parses_rooms_with_levels_rects_and_objects() {
        let mut b = b"LOTH".to_vec();
        b.extend(1u32.to_le_bytes());
        b.extend(0u32.to_le_bytes()); // no tile names
        for v in [8i32, 8, -2, 3] {
            b.extend(v.to_le_bytes());
        }
        b.extend(2u32.to_le_bytes()); // two rooms
        b.extend(b"hall\n");
        b.extend((-2i32).to_le_bytes());
        b.extend(1u32.to_le_bytes());
        for v in [10i32, 20, 3, 4] {
            b.extend(v.to_le_bytes());
        }
        b.extend(0u32.to_le_bytes());
        b.extend(b"kitchen\n");
        b.extend(0i32.to_le_bytes());
        b.extend(1u32.to_le_bytes());
        for v in [50i32, 60, 2, 2] {
            b.extend(v.to_le_bytes());
        }
        b.extend(1u32.to_le_bytes());
        for v in [7i32, 51, 61] {
            b.extend(v.to_le_bytes());
        }
        let h = parse_lotheader(&b).unwrap();
        let (rooms, end) = parse_rooms(&b, &h).unwrap();
        assert_eq!(end, b.len());
        assert_eq!((rooms[0].name.as_str(), rooms[0].level), ("hall", -2));
        assert!(rooms[0].contains(12, 23) && !rooms[0].contains(13, 23));
        assert_eq!(rooms[1].objects, vec![(7, 51, 61)]);
    }
    #[test]
    fn real_rosewood_cell_player_square_is_in_the_livingroom() {
        let p = std::path::Path::new("../../../media/maps/Muldraugh, KY/32_45.lotheader");
        let Ok(b) = std::fs::read(p) else { return };
        let h = parse_lotheader(&b).unwrap();
        let (rooms, _) = parse_rooms(&b, &h).unwrap();
        assert_eq!(rooms.len(), 362);
        // the real game puts the spawned character (x=8303.5, y=11689.5 -> cell-local 111,169, z=0) in a living room
        assert!(rooms
            .iter()
            .any(|r| r.level == 0 && r.name == "livingroom" && r.contains(111, 169)));
    }
}
