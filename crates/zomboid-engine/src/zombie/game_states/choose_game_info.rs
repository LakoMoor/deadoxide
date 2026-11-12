use std::path::Path;

use crate::java;

/// zombie.gameStates.ChooseGameInfo.Map: the fields of `media/maps/<dir>/map.info` the engine reads so far.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Map {
    pub title: String,
    pub lots_dir: Vec<String>,
    pub fixed2x: bool,
}

/// zombie.gameStates.ChooseGameInfo.getMapDetails: `None` when the folder has no map.info. Vanilla lookup only
/// (the original resolves the path through the mod-aware file system and caches the result).
pub fn get_map_details(media: &Path, dir: &str) -> Option<Map> {
    let text = crate::vfs::read(&media.join("maps").join(dir).join("map.info")).ok()?;
    let text = String::from_utf8_lossy(&text);
    let mut map = Map {
        title: dir.to_string(),
        ..Map::default()
    };
    for line in text.lines() {
        let line = java::trim(line);
        if line.starts_with("title=") {
            map.title = line.replace("title=", "");
        } else if line.starts_with("lots=") {
            map.lots_dir
                .push(java::trim(&line.replace("lots=", "")).to_string());
        } else if line.starts_with("fixed2x=") {
            // java.lang.Boolean.parseBoolean
            map.fixed2x = java::trim(&line.replace("fixed2x=", "")).eq_ignore_ascii_case("true");
        }
    }
    Some(map)
}
