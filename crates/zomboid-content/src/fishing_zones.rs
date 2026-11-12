//! Read fishing-zone records from version-249 map_zone.bin saves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FishingZone {
    pub caught: i32,
    pub x: i32,
    pub y: i32,
    pub z: i8,
    pub width: i32,
    pub height: i32,
    pub last_action: i32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FishingZones {
    pub string_count: usize,
    pub zone_count: usize,
    pub zones: Vec<FishingZone>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.offset.checked_add(n).ok_or("zone offset overflow")?;
        let result = self
            .bytes
            .get(self.offset..end)
            .ok_or("truncated zone file")?;
        self.offset = end;
        Ok(result)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn short(&mut self) -> Result<u16, String> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    fn int(&mut self) -> Result<i32, String> {
        let b = self.take(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn string(&mut self) -> Result<&'a [u8], String> {
        let n = self.short()?;
        self.take(usize::from(n))
    }
    fn count(&mut self) -> Result<usize, String> {
        usize::try_from(self.int()?).map_err(|_| "negative zone count".into())
    }
}

/// zombie.iso.IsoMetaGrid.loadZone / zombie.iso.zones.Zone.load, save version 249.
/// Only rectangle-shaped Fishing zones are exposed; other shapes cause an error.
pub fn parse_fishing_zones(bytes: &[u8]) -> Result<FishingZones, String> {
    let mut r = Reader { bytes, offset: 0 };
    if r.take(4)? != b"ZONE" || r.int()? != 249 {
        return Err("unsupported zone header/version".into());
    }
    let string_count = r.count()?;
    if string_count > bytes.len() / 2 {
        return Err("invalid zone string count".into());
    }
    let mut strings = Vec::with_capacity(string_count);
    for _ in 0..string_count {
        strings.push(r.string()?);
    }
    let zone_count = r.count()?;
    if zone_count > bytes.len() / 42 {
        return Err("invalid zone count".into());
    }
    let mut zones = Vec::new();
    for _ in 0..zone_count {
        let name_id = usize::from(r.short()?);
        let type_id = usize::from(r.short()?);
        let name = strings.get(name_id).ok_or("invalid zone name index")?;
        let kind = strings.get(type_id).ok_or("invalid zone type index")?;
        let x = r.int()?;
        let y = r.int()?;
        let z = r.byte()? as i8;
        let width = r.int()?;
        let height = r.int()?;
        let geometry = r.byte()?;
        if geometry > 3 {
            return Err("unsupported zone geometry".into());
        }
        if geometry != 0 {
            if geometry == 2 {
                r.byte()?;
            }
            let points = r.short()? as i16;
            let points = usize::try_from(points).map_err(|_| "negative zone point count")?;
            r.take(points.checked_mul(4).ok_or("zone point count overflow")?)?;
        }
        r.int()?;
        r.byte()?;
        let last_action = r.int()?;
        let original_name = usize::from(r.short()?);
        if original_name >= strings.len() {
            return Err("invalid original zone name index".into());
        }
        r.take(16)?;
        if *kind == b"Fishing" {
            if geometry != 0 {
                return Err("non-rectangle fishing zone is unsupported".into());
            }
            let caught = std::str::from_utf8(name)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            zones.push(FishingZone {
                caught,
                x,
                y,
                z,
                width,
                height,
                last_action,
            });
        }
    }
    let spawned = r.count()?;
    if spawned > bytes.len() / 6 {
        return Err("invalid spawned zone count".into());
    }
    for _ in 0..spawned {
        r.string()?;
        let ids = r.count()?;
        r.take(ids.checked_mul(16).ok_or("zone UUID count overflow")?)?;
    }
    Ok(FishingZones {
        string_count,
        zone_count,
        zones,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_written_synthetic_zones() {
        let fixture = include_str!("../../../tests/golden/fishing_zone_synthetic.txt");
        let hex = fixture
            .lines()
            .next()
            .unwrap()
            .strip_prefix("SYNTH hex=")
            .unwrap();
        let bytes: Vec<u8> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
            .collect();
        let actual = parse_fishing_zones(&bytes).unwrap();
        assert_eq!((actual.string_count, actual.zone_count), (4, 3));
        let lines: Vec<_> = actual
            .zones
            .iter()
            .map(|z| {
                format!(
                    "SYNTH zone={},{},{},{},{},{},INVALID,{}",
                    z.caught, z.x, z.y, z.z, z.width, z.height, z.last_action
                )
            })
            .collect();
        assert_eq!(lines, fixture.lines().skip(1).collect::<Vec<_>>());
        assert!(parse_fishing_zones(&bytes[..bytes.len() - 1]).is_err());
    }
    #[test]
    #[ignore = "requires the original isolated game save"]
    fn original_saved_zones() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../profile/Saves/Sandbox/2026-11-07_22-34-13/map_zone.bin");
        let bytes = std::fs::read(path).expect("isolated save");
        let zones = parse_fishing_zones(&bytes).expect("zones");
        assert_eq!(
            (zones.string_count, zones.zone_count, zones.zones.len()),
            (155, 5497, 0)
        );
        assert!(
            include_str!("../../../tests/golden/fishing_saved_zones.txt")
                .contains("ZONEFILE version=249 strings=155 zones=5497")
        );
    }
}
