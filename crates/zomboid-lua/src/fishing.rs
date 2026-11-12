//! Partial fishing calculations for the original saved trash seed.
use mlua::{Lua, Result, Value};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

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

/// Persistent state read by zombie.iso.FishSchoolManager.load.
#[derive(Debug, PartialEq, Eq)]
pub struct FishingSave {
    pub seed: i32,
    pub trash_seed: i32,
    pub chum: Vec<ChumEntry>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChumEntry {
    pub key: i64,
    pub max_force_time: i32,
    pub end_time: i32,
}

/// Partial `zombie.iso.FishSchoolManager` chum state. Times are original calendar minutes;
/// the caller must provide a correctly converted game calendar and invoke expiration.
#[derive(Default)]
pub struct ChumState {
    entries: std::collections::HashMap<i64, ChumEntry>,
}

/// zombie.iso.FishSchoolManager.coordsToHash: signed paired-coordinate key.
pub fn coords_hash(x: i32, y: i32) -> i64 {
    let x1 = if x >= 0 {
        2_i64 * i64::from(x)
    } else {
        -2_i64 * i64::from(x) - 1
    };
    let y1 = if y >= 0 {
        2_i64 * i64::from(y)
    } else {
        -2_i64 * i64::from(y) - 1
    };
    let c = if x1 >= y1 {
        x1.wrapping_mul(x1).wrapping_add(x1).wrapping_add(y1)
    } else {
        x1.wrapping_add(y1.wrapping_mul(y1))
    } / 2;
    if (x < 0) == (y < 0) {
        c
    } else {
        c.wrapping_neg().wrapping_sub(1)
    }
}

impl ChumState {
    /// zombie.iso.FishSchoolManager.load: restore chum records, last duplicate wins.
    pub fn from_save(save: &FishingSave) -> Self {
        Self {
            entries: save
                .chum
                .iter()
                .map(|c| {
                    (
                        c.key,
                        ChumEntry {
                            key: c.key,
                            max_force_time: c.max_force_time,
                            end_time: c.end_time,
                        },
                    )
                })
                .collect(),
        }
    }

    /// Return a chum entry by world coordinate.
    pub fn entry(&self, x: i32, y: i32) -> Option<&ChumEntry> {
        self.entries.get(&coords_hash(x, y))
    }

    /// zombie.iso.FishSchoolManager.addChum: replace the coordinate's record.
    pub fn add(&mut self, x: i32, y: i32, force: i32, now: i32) {
        let key = coords_hash(x, y);
        let max_force_time = now.wrapping_add(100);
        self.entries.insert(
            key,
            ChumEntry {
                key,
                max_force_time,
                end_time: max_force_time.wrapping_add(force),
            },
        );
    }

    /// zombie.iso.FishSchoolManager.updateFishingData: expire after, not at, end time.
    pub fn expire(&mut self, now: i32) {
        self.entries.retain(|_, chum| now <= chum.end_time);
    }

    /// zombie.iso.FishSchoolManager.getFishAbundance: chum contribution only.
    /// Fish points, no-fish zones and sound suppression must be handled by the caller;
    /// this must not be presented as complete fish abundance.
    pub fn contribution(
        &self,
        x: i32,
        y: i32,
        now: i32,
        disabled: &std::collections::HashSet<i64>,
    ) -> i32 {
        let mut result = 0_i32;
        for i in -6_i32..=6 {
            for j in -6_i32..=6 {
                let px = x.wrapping_add(i);
                let py = y.wrapping_add(j);
                let key = coords_hash(px, py);
                let Some(chum) = self.entries.get(&key) else {
                    continue;
                };
                if disabled.contains(&key) {
                    continue;
                }
                let dx = px as f32 - x as f32;
                let dy = py as f32 - y as f32;
                let distance = ((dx * dx + dy * dy) as f64).sqrt();
                let amount: i32 = if distance <= 3.0 {
                    15
                } else if distance <= 4.5 {
                    7
                } else {
                    continue;
                };
                let value = if now > chum.max_force_time {
                    amount
                } else {
                    let progress = 100_i32.wrapping_sub(chum.max_force_time.wrapping_sub(now));
                    amount.wrapping_mul(progress) / 100
                };
                result = result.wrapping_add(value);
            }
        }
        result
    }
}

/// zombie.iso.FishSchoolManager.load: decode saved seeds and chum entries.
pub fn read_fishing_save(path: &Path) -> std::io::Result<FishingSave> {
    let data = std::fs::read(path)?;
    parse_fishing_save(&data)
}

fn parse_fishing_save(data: &[u8]) -> std::io::Result<FishingSave> {
    let invalid = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid fishingData.bin length or chum count",
        )
    };
    if data.len() < 12 {
        return Err(invalid());
    }
    let int = |bytes: &[u8]| -> std::io::Result<i32> {
        Ok(i32::from_be_bytes(bytes.try_into().map_err(|_| invalid())?))
    };
    let count = usize::try_from(int(&data[8..12])?).map_err(|_| invalid())?;
    let expected = count
        .checked_mul(16)
        .and_then(|n| n.checked_add(12))
        .ok_or_else(invalid)?;
    if data.len() < expected {
        return Err(invalid());
    }
    let mut chum = Vec::with_capacity(count);
    for entry in data[12..expected].chunks_exact(16) {
        chum.push(ChumEntry {
            key: i64::from_be_bytes(entry[0..8].try_into().map_err(|_| invalid())?),
            max_force_time: int(&entry[8..12])?,
            end_time: int(&entry[12..16])?,
        });
    }
    Ok(FishingSave {
        seed: int(&data[0..4])?,
        trash_seed: int(&data[4..8])?,
        chum,
    })
}

/// zombie.iso.FishSchoolManager.load: obtain the persistent seed used for trash.
pub fn saved_trash_seed(path: &Path) -> std::io::Result<i32> {
    Ok(read_fishing_save(path)?.trash_seed)
}

/// Runtime portion of FishSchoolManager supported by this port.
pub struct FishingRuntime {
    pub fish_seed: i32,
    no_fish_zones: Vec<[i32; 4]>,
    pub random: Option<zomboid_content::rng::CellularRng>,
    pub chum: ChumState,
    pub calendar_minutes: Option<i32>,
}

pub type SharedFishingRuntime = Rc<RefCell<FishingRuntime>>;

/// zombie.iso.FishSchoolManager.isNoFishZone: original lazily caches Lua zones.
fn no_fish_zones(lua: &Lua, fishing: &mut FishingRuntime) -> Result<bool> {
    if !fishing.no_fish_zones.is_empty() {
        return Ok(false);
    }
    let Value::Table(table) = lua.globals().get::<Value>("Fishing")? else {
        return Ok(true);
    };
    let Value::Table(zones) = table.get::<Value>("NoFishZones")? else {
        return Ok(true);
    };
    // Original loops raw indexes 0..size, exclusive of size; in a contiguous
    // 1-based table the final entry is therefore not loaded.
    for index in 0..zones.raw_len() {
        let value: Value = zones.raw_get(index)?;
        if let Value::Table(zone) = value {
            fishing.no_fish_zones.push([
                zone.get("x1")?,
                zone.get("y1")?,
                zone.get("x2")?,
                zone.get("y2")?,
            ]);
        }
    }
    Ok(false)
}

/// zombie.iso.FishSchoolManager.getFishAbundance: exact supported branch where
/// no eligible natural fish point requires unimplemented saved zone lookup.
fn fish_abundance(lua: &Lua, fishing: &mut FishingRuntime, x: i32, y: i32) -> Result<f64> {
    let all_excluded = no_fish_zones(lua, fishing)?;
    if !all_excluded {
        for i in -6_i32..=6 {
            for j in -6_i32..=6 {
                let px = x.wrapping_add(i);
                let py = y.wrapping_add(j);
                let excluded = fishing
                    .no_fish_zones
                    .iter()
                    .any(|z| px >= z[0] && px <= z[2] && py >= z[1] && py <= z[3]);
                if excluded
                    || random_float(px as i64, py as i64, fishing.fish_seed as i64) <= 0.995_f32
                {
                    continue;
                }
                let dx = px as f32 - x as f32;
                let dy = py as f32 - y as f32;
                let distance = ((dx * dx + dy * dy) as f64).sqrt();
                let radius = (random_float(
                    px as i64,
                    py as i64,
                    fishing.fish_seed.wrapping_add(6599) as i64,
                ) + 2.0_f32) as f64;
                if distance <= radius + 1.5 {
                    return Err(mlua::Error::runtime(
                        "fish abundance needs saved fishing zone state, not yet implemented",
                    ));
                }
            }
        }
    }
    let now = fishing
        .calendar_minutes
        .ok_or_else(|| mlua::Error::runtime("game calendar unavailable for fish abundance"))?;
    Ok(fishing
        .chum
        .contribution(x, y, now, &std::collections::HashSet::new())
        .max(0) as f64)
}

/// Install the supported FishSchoolManager methods with restored chum state.
pub fn install_saved(lua: &Lua, save: &FishingSave) -> Result<SharedFishingRuntime> {
    let state = Rc::new(RefCell::new(FishingRuntime {
        fish_seed: save.seed,
        no_fish_zones: Vec::new(),
        random: None,
        chum: ChumState::from_save(save),
        calendar_minutes: None,
    }));
    let manager = lua.create_table()?;
    let trash_seed = save.trash_seed;
    manager.set(
        "getTrashAbundance",
        lua.create_function(move |_, (_self, x, y): (Value, i32, i32)| {
            Ok(trash_abundance(trash_seed, x, y))
        })?,
    )?;
    let shared = state.clone();
    manager.set(
        "getFishAbundance",
        lua.create_function(move |lua, (_self, x, y): (Value, i32, i32)| {
            fish_abundance(lua, &mut shared.borrow_mut(), x, y)
        })?,
    )?;
    let shared = state.clone();
    manager.set(
        "addChum",
        lua.create_function(move |_, (_self, x, y, force): (Value, i32, i32, f64)| {
            let mut fishing = shared.borrow_mut();
            let now = fishing
                .calendar_minutes
                .ok_or_else(|| mlua::Error::runtime("game calendar unavailable for addChum"))?;
            fishing.chum.add(x, y, force as i32, now);
            Ok(())
        })?,
    )?;
    let shared = state.clone();
    manager.set(
        "updateSeed",
        lua.create_function(move |_, _self: Value| {
            let mut fishing = shared.borrow_mut();
            let rng = fishing.random.as_mut().ok_or_else(|| {
                mlua::Error::runtime(
                    "original shared RNG is unavailable; daily fishing seed was not updated",
                )
            })?;
            fishing.fish_seed = rng.next_int(100000);
            Ok(())
        })?,
    )?;
    let class = lua.create_table()?;
    class.set(
        "getInstance",
        lua.create_function(move |_, ()| Ok(manager.clone()))?,
    )?;
    lua.globals().set("FishSchoolManager", class)?;
    Ok(state)
}

/// Install the trash-only environment with no saved chum or clock.
pub fn install(lua: &Lua, trash_seed: i32) -> Result<()> {
    install_saved(
        lua,
        &FishingSave {
            seed: -1,
            trash_seed,
            chum: Vec::new(),
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires installed original fishing Lua"]
    fn real_lure_selection_uses_chum_and_matches_original() -> Result<()> {
        let lua = Lua::new();
        let state = install_saved(
            &lua,
            &FishingSave {
                seed: 1485,
                trash_seed: 41859,
                chum: vec![],
            },
        )?;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../media/lua/shared/Fishing");
        for name in ["FishingZones.lua", "Fish.lua"] {
            let source = std::fs::read_to_string(root.join(name)).map_err(mlua::Error::external)?;
            lua.load(&source).set_name(name).exec()?;
        }
        lua.load("Fishing.Utils={skillSizeLimit={[1]=100}}; Fishing.fishes={{name='fish',maxWeight=1,isPredator=false,isLake=true,lure={worm=1}}}; Fishing.trashItems={'trash'}; setGameSpeed=function() end; getGameTime=function() return {setMultiplier=function() end} end; ZombRandFloat=function() return 0.5 end; ZombRand=function() return 0 end; subject={x=8306,y=11692,fishingLvl=1,isRiver=false,isReel=false,lure='worm'}; function selectFish() local item,trash=Fishing.Fish.getFishByLure(subject); return type(item)=='table' and item.name or item,trash end").exec()?;
        let select: mlua::Function = lua.globals().get("selectFish")?;
        state.borrow_mut().calendar_minutes = Some(0);
        let (before, before_trash): (String, bool) = select.call(())?;
        lua.load("FishSchoolManager.getInstance():addChum(8306,11692,200)")
            .exec()?;
        state.borrow_mut().calendar_minutes = Some(60);
        let (after, after_trash): (String, bool) = select.call(())?;
        let actual =
            format!("LURE before={before},{before_trash}\nLURE after={after},{after_trash}\n");
        assert_eq!(
            actual,
            include_str!("../../../tests/golden/fishing_lure.txt")
        );
        Ok(())
    }

    #[test]
    #[ignore = "requires installed original FishingZones.lua"]
    fn original_fishing_zones_and_supported_abundance() -> Result<()> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../media/lua/shared/Fishing/FishingZones.lua");
        let source = std::fs::read_to_string(path).map_err(mlua::Error::external)?;
        let lua = Lua::new();
        let state = install_saved(
            &lua,
            &FishingSave {
                seed: 1485,
                trash_seed: 41859,
                chum: vec![],
            },
        )?;
        lua.load(&source).exec()?;
        assert!(!no_fish_zones(&lua, &mut state.borrow_mut())?);
        let zone_fixture = include_str!("../../../tests/golden/fishing_no_fish_zones.txt");
        let binding = state.borrow();
        let zones = &binding.no_fish_zones;
        assert_eq!(zones.len(), 13);
        for line in zone_fixture
            .lines()
            .filter_map(|line| line.strip_prefix("POINT "))
        {
            let (coordinates, expected) = line.split_once('=').unwrap();
            let (x, y) = coordinates.split_once(',').unwrap();
            let x: i32 = x.parse().unwrap();
            let y: i32 = y.parse().unwrap();
            let actual = zones
                .iter()
                .any(|z| x >= z[0] && x <= z[2] && y >= z[1] && y <= z[3]);
            assert_eq!(actual, expected.parse::<bool>().unwrap(), "{line}");
        }
        drop(binding);
        let action: mlua::Function = lua.load("return function(x,y) return FishSchoolManager.getInstance():getFishAbundance(x,y) end").eval()?;
        let mut checked = 0;
        for line in include_str!("../../../tests/golden/fishing_abundance_partial.txt").lines() {
            let (label, body) = line.split_once(' ').unwrap();
            let (input, expected) = body.split_once('=').unwrap();
            let mut fields = input.split(',');
            let seed = fields.next().unwrap().parse().unwrap();
            let x = fields.next().unwrap().parse().unwrap();
            let y = fields.next().unwrap().parse().unwrap();
            {
                let mut fish = state.borrow_mut();
                fish.fish_seed = seed;
                fish.chum = ChumState::default();
                fish.chum.add(x, y, 200, 0);
                fish.calendar_minutes = Some(60);
            }
            let result = action.call::<f64>((x, y));
            if label == "ABUND" {
                assert_eq!(result?, expected.parse::<f64>().unwrap(), "{line}");
            } else {
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("saved fishing zone state"),
                    "{line}"
                );
            }
            checked += 1;
        }
        assert_eq!(checked, 28);
        Ok(())
    }

    #[test]
    fn missing_lua_zones_exclude_fish_and_chum_is_visible() -> Result<()> {
        let lua = Lua::new();
        let state = install_saved(
            &lua,
            &FishingSave {
                seed: 1485,
                trash_seed: 0,
                chum: vec![],
            },
        )?;
        state.borrow_mut().calendar_minutes = Some(0);
        lua.load("FishSchoolManager.getInstance():addChum(100,100,200)")
            .exec()?;
        state.borrow_mut().calendar_minutes = Some(60);
        let value: f64 = lua
            .load("return FishSchoolManager.getInstance():getFishAbundance(100,100)")
            .eval()?;
        assert_eq!(value, 9.0);
        Ok(())
    }

    #[test]
    #[ignore = "requires installed original game Lua"]
    fn real_chum_action_matches_original_with_test_doubles() -> Result<()> {
        let lua = Lua::new();
        let state = install_saved(
            &lua,
            &FishingSave {
                seed: 1,
                trash_seed: 2,
                chum: vec![],
            },
        )?;
        state.borrow_mut().calendar_minutes = Some(12368760);
        lua.load("require=function() end; ISBaseTimedAction={derive=function() return {} end}; sendServerCommand=function(...) sent=true end; sendRemoveItemFromContainer=function(...) removed=true end").exec()?;
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../../media/lua/shared/TimedActions/Fishing/TimedActions/AddChumToWaterAction.lua",
        );
        let script = std::fs::read_to_string(path).map_err(mlua::Error::external)?;
        lua.load(&script).exec()?;
        lua.load("local item={getHungChange=function() return -0.10125 end}; local inv={Remove=function() end}; local obj={chum=item,square={getX=function() return 100 end,getY=function() return 100 end},character={getInventory=function() return inv end}}; success=AddChumToWaterAction.complete(obj)").exec()?;
        let globals = lua.globals();
        let success: bool = globals.get("success")?;
        let sent: bool = globals.get("sent")?;
        let removed: bool = globals.get("removed")?;
        let fish = state.borrow();
        let chum = fish.chum.entry(100, 100).expect("action added chum");
        let observed = format!(
            "ACTION success={success} sent={sent} removed={removed} max={} end={}\n",
            chum.max_force_time, chum.end_time
        );
        assert_eq!(
            observed,
            include_str!("../../../tests/golden/fishing_action.txt")
        );
        Ok(())
    }

    #[test]
    fn chum_matches_original_with_fish_points_excluded() {
        let mut chum = ChumState::default();
        // Only elapsed minutes matter here; original uses UTC calendar minutes.
        chum.add(100, 100, 200, 0);
        let disabled = std::collections::HashSet::new();
        let fixture = include_str!("../../../tests/golden/fishing_chum_behavior.txt");
        let mut count = 0;
        for line in fixture
            .lines()
            .filter(|line| line.starts_with("CHUM ") && line.contains(','))
        {
            let (input, expected) = line.strip_prefix("CHUM ").unwrap().split_once('=').unwrap();
            let mut fields = input.split(',');
            let hour: f32 = fields.next().unwrap().parse().unwrap();
            let x = fields.next().unwrap().parse().unwrap();
            let y = fields.next().unwrap().parse().unwrap();
            let minute = ((hour - 10.0) * 60.0) as i32;
            let actual = chum.contribution(x, y, minute, &disabled).max(0) as f64;
            assert_eq!(actual, expected.parse::<f64>().unwrap(), "{line}");
            count += 1;
        }
        assert_eq!(count, 42);
        for line in fixture
            .lines()
            .filter_map(|line| line.strip_prefix("HASH "))
        {
            let (input, output) = line.split_once('=').unwrap();
            let (x, y) = input.split_once(',').unwrap();
            assert_eq!(
                coords_hash(x.parse().unwrap(), y.parse().unwrap()),
                output.parse().unwrap(),
                "{line}"
            );
        }
        let restored = ChumState::from_save(&FishingSave {
            seed: 0,
            trash_seed: 0,
            chum: vec![ChumEntry {
                key: coords_hash(100, 100),
                max_force_time: 100,
                end_time: 300,
            }],
        });
        assert_eq!(
            restored.contribution(100, 100, 60, &disabled),
            chum.contribution(100, 100, 60, &disabled)
        );
        let mut multiple = ChumState::default();
        multiple.add(100, 100, 200, 0);
        multiple.add(104, 100, 200, 0);
        for case in ["combined", "replaced"] {
            if case == "replaced" {
                multiple.add(104, 100, 200, 0);
            }
            let prefix = format!("CHUM {case}=");
            let expected: f64 = fixture
                .lines()
                .find_map(|line| line.strip_prefix(&prefix))
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(
                multiple.contribution(102, 100, 60, &disabled) as f64,
                expected
            );
        }
        let mut blocked = std::collections::HashSet::new();
        blocked.insert(coords_hash(100, 100));
        let noise: f64 = fixture
            .lines()
            .find_map(|line| line.strip_prefix("CHUM noise="))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            chum.contribution(100, 100, 60, &blocked).max(0) as f64,
            noise
        );
        chum.expire(300);
        assert_eq!(chum.contribution(100, 100, 300, &disabled), 15);
        chum.expire(301);
        assert_eq!(chum.contribution(100, 100, 301, &disabled), 0);
    }

    #[test]
    fn parses_original_written_chum_data() {
        let fixture = include_str!("../../../tests/golden/fishing_chum_save.txt");
        let hex = fixture
            .lines()
            .next()
            .unwrap()
            .strip_prefix("CHUM hex=")
            .unwrap();
        let bytes: Vec<u8> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let save = parse_fishing_save(&bytes).unwrap();
        assert_eq!((save.seed, save.trash_seed), (12345, 54321));
        let mut actual: Vec<_> = save
            .chum
            .iter()
            .map(|entry| (entry.key, entry.max_force_time, entry.end_time))
            .collect();
        actual.sort_unstable();
        let expected: Vec<_> = fixture
            .lines()
            .filter_map(|line| line.strip_prefix("CHUM entry="))
            .map(|line| {
                let mut f = line.split(',');
                (
                    f.next().unwrap().parse::<i64>().unwrap(),
                    f.next().unwrap().parse::<i32>().unwrap(),
                    f.next().unwrap().parse::<i32>().unwrap(),
                )
            })
            .collect();
        assert_eq!(actual, expected);
    }

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
    #[ignore = "requires the isolated original-game save"]
    fn isolated_original_save_matches_java_load() -> Result<()> {
        let path = std::env::var_os("PZ_FISHING_SAVE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../profile/Saves/Sandbox/2026-11-07_22-34-13/fishingData.bin")
            });
        let seed = saved_trash_seed(&path).map_err(mlua::Error::external)?;
        let lua = Lua::new();
        install(&lua, seed)?;
        let fixture = include_str!("../../../tests/golden/fishing_saved.txt");
        let expected_seed: i32 = fixture
            .lines()
            .next()
            .unwrap()
            .strip_prefix("SAVE seeds=")
            .unwrap()
            .split_once(',')
            .unwrap()
            .1
            .parse()
            .unwrap();
        assert_eq!(seed, expected_seed);
        let function: mlua::Function = lua.load("return function(x,y) return FishSchoolManager.getInstance():getTrashAbundance(x,y) end").eval()?;
        let mut count = 0;
        for line in fixture
            .lines()
            .filter_map(|line| line.strip_prefix("SAVE trash="))
        {
            let mut fields = line.split(',');
            let x: i32 = fields.next().unwrap().parse().unwrap();
            let y: i32 = fields.next().unwrap().parse().unwrap();
            let expected = u64::from_str_radix(fields.next().unwrap(), 16).unwrap();
            let actual: f64 = function.call((x, y))?;
            assert_eq!(actual.to_bits(), expected, "{line}");
            count += 1;
        }
        assert_eq!(count, 6);
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
