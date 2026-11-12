//! Partial fishing calculations for the original saved trash seed.
use mlua::{Lua, Result, Value};
use std::path::Path;

/// zombie.iso.FishSchoolManager.procedureRandomFloat: Java signed wrapping and f64-to-f32 rounding.
fn random_float(x: i64, y: i64, seed: i64) -> f32 {
    let x = x.wrapping_shl(13) ^ x;
    let y = y.wrapping_shl(13) ^ y;
    let s = seed.wrapping_shl(13) ^ seed;
    let cube = |v: i64| v.wrapping_mul(v).wrapping_mul(v);
    let terms = [
        x.wrapping_mul(790169),
        cube(x).wrapping_mul(15731),
        y.wrapping_mul(789221),
        cube(y).wrapping_mul(16057),
        s.wrapping_mul(788317),
        cube(s).wrapping_mul(15401),
        x.wrapping_mul(y).wrapping_mul(209123),
        y.wrapping_mul(s).wrapping_mul(209581),
        x.wrapping_mul(s).wrapping_mul(208501),
        x.wrapping_mul(y).wrapping_mul(s).wrapping_mul(15749),
        1376312588,
    ];
    let t = terms.into_iter().fold(0_i64, i64::wrapping_add);
    (((t % 0x40000000) as f64 / 536870912.0 + 2.0) / 4.0) as f32
}

/// zombie.iso.FishSchoolManager.getTrashAbundance: uses only the persistent trash seed.
pub fn trash_abundance(seed: i32, x: i32, y: i32) -> f64 {
    for i in -6_i32..=6 {
        for j in -6_i32..=6 {
            let px = x.wrapping_add(i);
            let py = y.wrapping_add(j);
            let noise =
                |offset: i32| random_float(px as i64, py as i64, seed.wrapping_add(offset) as i64);
            if noise(9281) <= 0.99_f32 {
                continue;
            }
            let dx = px as f32 - x as f32;
            let dy = py as f32 - y as f32;
            let distance = ((dx * dx + dy * dy) as f64).sqrt();
            if distance < (4.0_f32 * noise(8573) + 2.0) as f64 {
                return noise(9601) as f64 / 2.0 + 0.05;
            }
        }
    }
    0.05
}

/// zombie.iso.FishSchoolManager.load: read the seeds from a complete fishingData.bin.
/// Chum entries are validated but not used by the trash abundance calculation.
pub fn saved_trash_seed(path: &Path) -> std::io::Result<i32> {
    let data = std::fs::read(path)?;
    let invalid = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid fishingData.bin length or chum count",
        )
    };
    if data.len() < 12 {
        return Err(invalid());
    }
    let count = i32::from_be_bytes(data[8..12].try_into().map_err(|_| invalid())?);
    let count = usize::try_from(count).map_err(|_| invalid())?;
    let expected = count
        .checked_mul(16)
        .and_then(|n| n.checked_add(12))
        .ok_or_else(invalid)?;
    if data.len() < expected {
        return Err(invalid());
    }
    Ok(i32::from_be_bytes(
        data[4..8].try_into().map_err(|_| invalid())?,
    ))
}

/// Install only the supported FishSchoolManager method; other fishing APIs are absent.
pub fn install(lua: &Lua, trash_seed: i32) -> Result<()> {
    let manager = lua.create_table()?;
    manager.set(
        "getTrashAbundance",
        lua.create_function(move |_, (_self, x, y): (Value, i32, i32)| {
            Ok(trash_abundance(trash_seed, x, y))
        })?,
    )?;
    let class = lua.create_table()?;
    class.set(
        "getInstance",
        lua.create_function(move |_, ()| Ok(manager.clone()))?,
    )?;
    lua.globals().set("FishSchoolManager", class)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_data_seed_reaches_lua() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("pz-fishing-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(mlua::Error::external)?;
        let path = dir.join("fishingData.bin");
        let bytes = [0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 0];
        std::fs::write(&path, bytes).map_err(mlua::Error::external)?;
        let seed = saved_trash_seed(&path).map_err(mlua::Error::external)?;
        std::fs::write(&path, [0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 1])
            .map_err(mlua::Error::external)?;
        assert_eq!(
            saved_trash_seed(&path).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        std::fs::write(&path, [0, 0, 0, 42, 0, 0, 0, 0, 255, 255, 255, 255])
            .map_err(mlua::Error::external)?;
        assert_eq!(
            saved_trash_seed(&path).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        let lua = Lua::new();
        install(&lua, seed)?;
        let actual: f64 = lua
            .load(
                "return FishSchoolManager.getInstance():getTrashAbundance(2147483600,-2147483600)",
            )
            .eval()?;
        assert_eq!(actual.to_bits(), 0x3fb8cafc0ccccccd);
        std::fs::remove_dir_all(dir).map_err(mlua::Error::external)?;
        Ok(())
    }

    #[test]
    fn trash_abundance_matches_original() {
        let mut count = 0;
        let mut non_default = 0;
        for line in include_str!("../../../tests/golden/fishing_trash.txt").lines() {
            let (input, bits) = line
                .strip_prefix("TRASH ")
                .unwrap()
                .split_once('=')
                .unwrap();
            let values: Vec<i32> = input.split(',').map(|s| s.parse().unwrap()).collect();
            let expected = u64::from_str_radix(bits, 16).unwrap();
            assert_eq!(
                trash_abundance(values[0], values[1], values[2]).to_bits(),
                expected,
                "{line}"
            );
            non_default += usize::from(expected != 0.05_f64.to_bits());
            count += 1;
        }
        assert_eq!(count, 75);
        assert!(non_default > 0);
    }
}
