//! Measured subset of version 249 sandbox saves (`map_sand.bin`).
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedDayLength {
    pub option: u8,
    pub minutes: u32,
}

/// zombie.SandboxOptions.getDayLengthMinutes: measured mappings for options 1 through 27.
pub fn day_length_minutes(option: u8) -> Option<u32> {
    match option {
        1 => Some(15),
        2 => Some(30),
        3 => Some(60),
        4 => Some(90),
        5 => Some(120),
        6 => Some(180),
        7..=27 => Some((option as u32 - 3) * 60),
        _ => None,
    }
}

/// zombie.SandboxOptions.load: read the DayLength entry from the measured version-249 prefix.
/// Other options and the trailing data are skipped, not interpreted or fully validated.
pub fn parse_day_length(bytes: &[u8]) -> Result<SavedDayLength, String> {
    if bytes.len() < 16 || &bytes[..4] != b"SAND" {
        return Err("invalid or truncated sandbox header".into());
    }
    let word = |i: usize| u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    if word(4) != 249 || word(8) != 6 {
        return Err(format!(
            "unsupported sandbox version {}/{}",
            word(4),
            word(8)
        ));
    }
    let count = word(12) as usize;
    if count > (bytes.len() - 16) / 4 {
        return Err("invalid sandbox option count".into());
    }
    let mut offset = 16usize;
    let read_string = |offset: &mut usize| -> Result<&[u8], String> {
        let length = bytes
            .get(*offset..*offset + 2)
            .ok_or("truncated sandbox string length")?;
        *offset += 2;
        let end = offset
            .checked_add(u16::from_be_bytes([length[0], length[1]]) as usize)
            .ok_or("sandbox string length overflow")?;
        let value = bytes.get(*offset..end).ok_or("truncated sandbox string")?;
        *offset = end;
        Ok(value)
    };
    let mut option = None;
    for _ in 0..count {
        let key = read_string(&mut offset)?;
        let value = read_string(&mut offset)?;
        if key == b"DayLength" {
            if option.is_some() {
                return Err("duplicate DayLength option".into());
            }
            let value = std::str::from_utf8(value).map_err(|_| "non-ASCII DayLength")?;
            option = Some(value.parse::<u8>().map_err(|_| "invalid DayLength value")?);
        }
    }
    let option = option.ok_or("missing DayLength option")?;
    let minutes = day_length_minutes(option).ok_or("unsupported DayLength value")?;
    Ok(SavedDayLength { option, minutes })
}

pub fn load_day_length(path: &Path) -> Result<SavedDayLength, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_day_length(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn synthetic(option: &str) -> Vec<u8> {
        let mut data = b"SAND\0\0\0\xf9\0\0\0\x06\0\0\0\x02".to_vec();
        for value in ["Unknown", "irrelevant", "DayLength", option] {
            data.extend_from_slice(&(value.len() as u16).to_be_bytes());
            data.extend_from_slice(value.as_bytes());
        }
        data
    }
    #[test]
    fn day_length_matches_original_oracle() {
        let fixture = include_str!("../../../tests/golden/sandbox_day_length.txt");
        let mut count = 0;
        for line in fixture.lines().filter(|s| s.starts_with("OPTION ")) {
            let (option, minutes) = line[7..].split_once('=').expect("oracle line");
            let option: u8 = option.parse().expect("option");
            assert_eq!(
                day_length_minutes(option),
                Some(minutes.parse().expect("minutes"))
            );
            count += 1;
        }
        assert_eq!(count, 27);
        for line in fixture.lines().filter(|s| s.starts_with("LOAD ")) {
            let (option, expected) = line[5..].split_once('=').expect("oracle line");
            let (value, minutes) = expected.split_once(',').expect("oracle values");
            assert_eq!(
                parse_day_length(&synthetic(option)).expect("sandbox"),
                SavedDayLength {
                    option: value.parse().expect("option"),
                    minutes: minutes.parse().expect("minutes")
                }
            );
        }
        assert!(parse_day_length(&synthetic("28")).is_err());
        assert!(parse_day_length(&synthetic("4")[..18]).is_err());
        assert!(parse_day_length(b"SAND").is_err());
    }
    #[test]
    #[ignore = "requires the user's isolated original-game save"]
    fn real_save_day_length_matches_original() {
        let path = Path::new("../../profile/Saves/Sandbox/2026-11-07_22-34-13/map_sand.bin");
        let result = load_day_length(path).expect("isolated original save");
        assert_eq!(
            result,
            SavedDayLength {
                option: 4,
                minutes: 90
            }
        );
    }
}
