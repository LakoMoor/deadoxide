//! Original cellular random generator, using rules read from the player's JAR.
use std::io::{self, Read};
use std::path::Path;

const CLASS: &str = "org/uncommons/maths/random/CellularAutomatonRNG.class";
const CELLS: usize = 2056;
const RULES: usize = 511;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

// Read a JVM small integer constant from an array-initializer instruction.
fn constant(code: &[u8], pos: &mut usize) -> Option<i32> {
    let op = *code.get(*pos)?;
    *pos += 1;
    match op {
        0x02..=0x08 => Some(i32::from(op) - 3),
        0x10 => {
            let n = *code.get(*pos)? as i8;
            *pos += 1;
            Some(i32::from(n))
        }
        0x11 => {
            let n = i16::from_be_bytes(code.get(*pos..*pos + 2)?.try_into().ok()?);
            *pos += 2;
            Some(i32::from(n))
        }
        _ => None,
    }
}

// Extract only the verified straight-line int-array initializer. Refuse unfamiliar
// bytecode instead of silently inventing a lookup table for another game version.
fn rule_from_class(class: &[u8]) -> io::Result<[u8; RULES]> {
    if !class.starts_with(&[0xca, 0xfe, 0xba, 0xbe]) {
        return Err(invalid("not a Java class"));
    }
    let marker = [0x11, 0x01, 0xff, 0xbc, 0x0a];
    let mut matches = Vec::new();
    for start in 0..class.len().saturating_sub(marker.len()) {
        if class[start..].starts_with(&marker) {
            let mut pos = start + marker.len();
            let mut result = [0_u8; RULES];
            let mut valid = true;
            for (index, slot) in result.iter_mut().enumerate() {
                if class.get(pos) != Some(&0x59) {
                    valid = false;
                    break;
                }
                pos += 1;
                let idx = constant(class, &mut pos);
                let value = constant(class, &mut pos);
                if idx != Some(index as i32)
                    || class.get(pos) != Some(&0x4f)
                    || !matches!(value, Some(0..=255))
                {
                    valid = false;
                    break;
                }
                *slot = value.unwrap_or_default() as u8;
                pos += 1;
            }
            if valid && class.get(pos) == Some(&0xb3) && result[256..] == result[..255] {
                let mut seen = [false; 256];
                for &v in &result[..256] {
                    seen[v as usize] = true;
                }
                if seen.iter().all(|v| *v) {
                    matches.push(result);
                }
            }
        }
    }
    if matches.len() != 1 {
        return Err(invalid(
            "unsupported or ambiguous cellular RNG rule initializer",
        ));
    }
    Ok(matches.remove(0))
}

/// Read the original RNG rule from the installed game's `projectzomboid.jar`.
pub fn load_rule(game_root: &Path) -> io::Result<[u8; RULES]> {
    let file = std::fs::File::open(game_root.join("projectzomboid.jar"))?;
    let mut jar = zip::ZipArchive::new(file).map_err(io::Error::other)?;
    let class = jar.by_name(CLASS).map_err(io::Error::other)?;
    if class.size() > 1024 * 1024 {
        return Err(invalid("RNG class is too large"));
    }
    let mut bytes = Vec::new();
    class.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err(invalid("RNG class is too large"));
    }
    rule_from_class(&bytes)
}

/// org.uncommons.maths.random.CellularAutomatonRNG: explicit seed and original rule.
pub struct CellularRng {
    rule: [u8; RULES],
    cells: [u8; CELLS],
    index: usize,
}

impl CellularRng {
    /// org.uncommons.maths.random.CellularAutomatonRNG constructor and warm-up.
    pub fn new(seed: [u8; 4], rule: [u8; RULES]) -> Self {
        let mut cells = [0_u8; CELLS];
        for i in 0..4 {
            cells[CELLS - 1 - i] = (i16::from(seed[i] as i8) + 128) as u8;
        }
        let value = i32::from_be_bytes(seed);
        let value = if value == -1 {
            value
        } else {
            value.wrapping_add(1)
        };
        for (i, cell) in cells[..CELLS - 4].iter_mut().enumerate() {
            *cell = (value >> (i % 32)) as u8;
        }
        let mut rng = Self {
            rule,
            cells,
            index: CELLS - 1,
        };
        for _ in 0..0x102010 {
            rng.next_bits(32);
        }
        rng
    }

    /// org.uncommons.maths.random.CellularAutomatonRNG.next.
    fn next_bits(&mut self, bits: u32) -> u32 {
        let d = self.index;
        let c = d - 1;
        let b = c - 1;
        let a = b - 1;
        self.cells[d] = self.rule[usize::from(self.cells[c]) + usize::from(self.cells[d])];
        self.cells[c] = self.rule[usize::from(self.cells[b]) + usize::from(self.cells[c])];
        self.cells[b] = self.rule[usize::from(self.cells[a]) + usize::from(self.cells[b])];
        self.cells[a] = self.rule[usize::from(self.cells[a])
            + if a == 0 {
                0
            } else {
                usize::from(self.cells[a - 1])
            }];
        self.index = if a == 0 { CELLS - 1 } else { d - 4 };
        let value =
            u32::from_le_bytes([self.cells[a], self.cells[b], self.cells[c], self.cells[d]]);
        value >> (32 - bits)
    }

    /// zombie.core.random.RandAbstract.Next(int), including Java Random.nextInt rejection.
    pub fn next_int(&mut self, max: i32) -> i32 {
        if max <= 0 {
            return 0;
        }
        if (max & -max) == max {
            return ((i64::from(max) * i64::from(self.next_bits(31))) >> 31) as i32;
        }
        loop {
            let bits = self.next_bits(31) as i32;
            let value = bits % max;
            if bits.wrapping_sub(value).wrapping_add(max - 1) >= 0 {
                return value;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_bad_class() {
        assert!(rule_from_class(b"not a class").is_err());
    }

    #[test]
    #[ignore = "requires the installed original game JAR"]
    fn original_cellular_random_sequences() -> io::Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let rule = load_rule(&root)?;
        let mut current_seed = None;
        let mut rng = None;
        let mut count = 0;
        for line in include_str!("../../../tests/golden/cellular_rng.txt").lines() {
            let (input, output) = line.strip_prefix("RNG ").unwrap().split_once('=').unwrap();
            let mut fields = input.split(',');
            let seed = u32::from_str_radix(fields.next().unwrap(), 16).unwrap();
            let _index: usize = fields.next().unwrap().parse().unwrap();
            let bound: i32 = fields.next().unwrap().parse().unwrap();
            if current_seed != Some(seed) {
                rng = Some(CellularRng::new(seed.to_be_bytes(), rule));
                current_seed = Some(seed);
            }
            assert_eq!(
                rng.as_mut().unwrap().next_int(bound),
                output.parse().unwrap(),
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 70);
        Ok(())
    }
}
