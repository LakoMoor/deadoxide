//! Saved game: the local player row of `players.db` (SQLite) in a save folder (build 42.21.0, table `localPlayers`).
//! Reads use a read-only SQLite connection, which includes committed WAL data. SQLite may use
//! or create transient WAL shared-memory sidecars; this does not restore other saved world state.
#[cfg(feature = "native")]
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct SavedPlayer {
    pub id: i64,
    pub name: String,
    /// world square coordinates (x east, y south), fractional inside the square
    pub x: f64,
    pub y: f64,
    pub z: i32,
    pub dead: bool,
}

/// Reads the first local player. `Ok(None)` if the table has no rows.
#[cfg(feature = "native")]
pub fn load_player(db: &Path) -> Result<Option<SavedPlayer>, String> {
    let c = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let mut st = c
        .prepare("select id, name, x, y, z, isDead from localPlayers order by id limit 1")
        .map_err(|e| e.to_string())?;
    let mut rows = st.query([]).map_err(|e| e.to_string())?;
    match rows.next().map_err(|e| e.to_string())? {
        None => Ok(None),
        Some(r) => Ok(Some(SavedPlayer {
            id: r.get(0).map_err(|e| e.to_string())?,
            name: r.get(1).map_err(|e| e.to_string())?,
            x: r.get(2).map_err(|e| e.to_string())?,
            y: r.get(3).map_err(|e| e.to_string())?,
            z: r.get::<_, f64>(4).map_err(|e| e.to_string())? as i32,
            dead: r.get::<_, i64>(5).map_err(|e| e.to_string())? != 0,
        })),
    }
}

/// Prefix of build 42.21.0 `map_t.bin`. Month and day retain the original's zero-based values.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedGameTime {
    pub version: i32,
    pub time_of_day: f32,
    pub nights_survived: i32,
    pub year: i32,
    pub month_index: i32,
    pub day_index: i32,
}

impl SavedGameTime {
    /// zombie.GameTime.getHour: integer hour for a validated time in [0, 24).
    pub fn hour(&self) -> u32 {
        self.time_of_day as u32
    }

    /// zombie.GameTime.getMinutes: preserve the original f32 subtraction and truncation.
    pub fn minutes(&self) -> u32 {
        ((self.time_of_day - self.hour() as f32) * 60.0) as u32
    }
}

/// zombie.GameTime.load: read measured time, date and nights fields of a version 249 save.
/// This does not parse or validate the remainder of the file.
pub fn parse_game_time_prefix(bytes: &[u8]) -> Result<SavedGameTime, String> {
    if bytes.len() < 40 {
        return Err("map_t.bin is shorter than the time header".into());
    }
    if &bytes[..4] != b"GMTM" {
        return Err("map_t.bin has an unsupported header".into());
    }
    let i32_at =
        |i: usize| i32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    let version = i32_at(4);
    if version != 249 {
        return Err(format!("unsupported map_t.bin version {version}"));
    }
    let time_of_day = f32::from_bits(i32_at(24) as u32);
    if !time_of_day.is_finite() || !(0.0..24.0).contains(&time_of_day) {
        return Err("map_t.bin contains an unsupported time of day".into());
    }
    Ok(SavedGameTime {
        version,
        time_of_day,
        nights_survived: i32_at(12),
        day_index: i32_at(28),
        month_index: i32_at(32),
        year: i32_at(36),
    })
}

/// Reads the measured time prefix without modifying the save.
pub fn load_game_time(path: &Path) -> Result<SavedGameTime, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_game_time_prefix(&bytes)
}

/// The 256x256 map cell that contains a world position.
pub fn cell_of(x: f64, y: f64) -> (i32, i32) {
    ((x / 256.0).floor() as i32, (y / 256.0).floor() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn time_header_matches_original_controlled_probe() {
        // Minimal synthetic prefix: version 249 and the original-observed fields.
        let mut original = [0u8; 40];
        original[..4].copy_from_slice(b"GMTM");
        for (offset, bits) in [
            (4, 249u32),
            (8, 1.0f32.to_bits()),
            (16, 750),
            (20, 0x4136c9cb),
            (24, 0x4136ca18),
            (28, 8),
            (32, 6),
            (36, 1993),
        ] {
            original[offset..offset + 4].copy_from_slice(&bits.to_be_bytes());
        }
        let fixture = include_str!("../../../tests/golden/save_time_probe.txt");
        let mut count = 0;
        for line in fixture.lines() {
            let (name, output) = line
                .strip_prefix("PROBE ")
                .expect("probe line")
                .split_once('=')
                .expect("probe fields");
            let expected: Vec<_> = output.split(',').collect();
            assert_eq!(expected.len(), 7);
            let mut bytes = original;
            let patch: Option<(usize, u32)> = match name {
                "original" => None,
                "current" => Some((24, 19.75f32.to_bits())),
                "previous" => Some((20, 3.5f32.to_bits())),
                "day" => Some((28, 17)),
                "month" => Some((32, 2)),
                "year" => Some((36, 1998)),
                _ => Some((
                    24,
                    name.strip_prefix("time-")
                        .expect("time case")
                        .parse::<f32>()
                        .expect("float")
                        .to_bits(),
                )),
            };
            if let Some((offset, value)) = patch {
                bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
            }
            let t = parse_game_time_prefix(&bytes).expect("time prefix");
            assert_eq!(
                t.time_of_day.to_bits(),
                u32::from_str_radix(expected[0], 16).unwrap(),
                "{name}"
            );
            assert_eq!(t.hour(), expected[1].parse().unwrap(), "{name}");
            assert_eq!(t.minutes(), expected[2].parse().unwrap(), "{name}");
            assert_eq!(t.year, expected[3].parse().unwrap(), "{name}");
            assert_eq!(t.month_index, expected[4].parse().unwrap(), "{name}");
            assert_eq!(t.day_index, expected[5].parse().unwrap(), "{name}");
            count += 1;
        }
        assert_eq!(count, 11);
        assert!(parse_game_time_prefix(&original[..39]).is_err());
        original[7] = 0;
        assert!(parse_game_time_prefix(&original).is_err());
    }

    #[test]
    fn saved_nights_match_original_probe() {
        let mut header = [0u8; 40];
        header[..4].copy_from_slice(b"GMTM");
        header[4..8].copy_from_slice(&249i32.to_be_bytes());
        header[24..28].copy_from_slice(&11.0f32.to_bits().to_be_bytes());
        let mut count = 0;
        for line in include_str!("../../../tests/golden/save_nights.txt").lines() {
            let (input, result) = line
                .strip_prefix("NIGHTS ")
                .unwrap()
                .split_once('=')
                .unwrap();
            let expected = result.split(',').next().unwrap().parse::<i32>().unwrap();
            header[12..16].copy_from_slice(&input.parse::<i32>().unwrap().to_be_bytes());
            assert_eq!(
                parse_game_time_prefix(&header).unwrap().nights_survived,
                expected
            );
            count += 1;
        }
        assert_eq!(count, 4);
    }

    #[test]
    fn cell_of_matches_the_known_spawn() {
        assert_eq!(cell_of(8303.5, 11689.5), (32, 45));
        assert_eq!(cell_of(-0.5, 255.9), (-1, 0));
    }
    #[test]
    fn missing_db_is_an_error_not_a_panic() {
        assert!(load_player(Path::new("no/such/players.db")).is_err());
    }
    #[test]
    fn real_save_player_stands_in_the_livingroom() {
        // a real save written by the game in the isolated profile; skipped where it does not exist
        let dir = Path::new("../../profile/Saves/Sandbox");
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let Some(db) = rd
            .flatten()
            .map(|e| e.path().join("players.db"))
            .find(|p| p.exists())
        else {
            return;
        };
        let tmp = std::env::temp_dir().join("zomboid_rs_players_test.db");
        std::fs::copy(&db, &tmp).unwrap();
        let p = load_player(&tmp).unwrap().expect("a saved player");
        assert!(!p.dead);
        let (cx, cy) = cell_of(p.x, p.y);
        // independent oracle: the room table of the cell header puts that square inside a living room (where the game showed the character)
        let hdr =
            Path::new("../../../media/maps/Muldraugh, KY").join(format!("{cx}_{cy}.lotheader"));
        let Ok(b) = std::fs::read(hdr) else { return };
        let h = crate::lot::parse_lotheader(&b).unwrap();
        let (rooms, _) = crate::lot::parse_rooms(&b, &h).unwrap();
        let (lx, ly) = (
            (p.x.floor() as i32).rem_euclid(256),
            (p.y.floor() as i32).rem_euclid(256),
        );
        assert!(
            rooms
                .iter()
                .any(|r| r.level == p.z && r.name == "livingroom" && r.contains(lx, ly)),
            "player at ({lx},{ly}) z{} not in a livingroom",
            p.z
        );
    }
}
