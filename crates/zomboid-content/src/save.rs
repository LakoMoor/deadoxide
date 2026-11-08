//! Saved game: the local player row of `players.db` (SQLite) in a save folder (build 42.21.0, table `localPlayers`).
//! Opened read-only; callers should point it at a COPY of a save that a running game may hold open.
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

/// The 256x256 map cell that contains a world position.
pub fn cell_of(x: f64, y: f64) -> (i32, i32) {
    ((x / 256.0).floor() as i32, (y / 256.0).floor() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
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
