//! zombie.iso.IsoLot: little-endian readers shared by the map loaders (chunk loading follows).

/// Sequential reader over a whole file, standing in for the original's `BufferedRandomAccessFile`.
pub struct RandomAccess<'a> {
    pub b: &'a [u8],
    pub p: usize,
}
impl<'a> RandomAccess<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        RandomAccess { b, p: 0 }
    }
    /// java.io.RandomAccessFile.read(): -1 at end of file.
    pub fn read(&mut self) -> i32 {
        match self.b.get(self.p) {
            Some(c) => {
                self.p += 1;
                *c as i32
            }
            None => -1,
        }
    }
    /// java.io.RandomAccessFile.seek
    pub fn seek(&mut self, pos: usize) {
        self.p = pos;
    }
    /// java.io.RandomAccessFile.read(byte[]): number of bytes read, fewer than asked at end of file.
    pub fn read_bytes(&mut self, n: usize) -> &'a [u8] {
        let end = (self.p + n).min(self.b.len());
        let s = &self.b[self.p.min(end)..end];
        self.p = end;
        s
    }
}

/// zombie.iso.IsoLot.readInt: `Err` is EOFException.
pub fn read_int(input: &mut RandomAccess) -> Result<i32, String> {
    match input.b.get(input.p..input.p + 4) {
        Some(s) => {
            input.p += 4;
            Ok(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        }
        None => {
            input.p = input.b.len();
            Err("java.io.EOFException".into())
        }
    }
}

/// zombie.iso.IsoLot.readString (BufferedRandomAccessFile.getNextLine): text up to '\n' as UTF-8, a '\r' before it
/// dropped; `None` (Java null) at end of file. Differs from the original only for a line that straddles its 4096-byte
/// buffer, which it decodes byte-per-char and keeps the '\r' of.
pub fn read_string(input: &mut RandomAccess) -> Option<String> {
    let rest = input.b.get(input.p..).unwrap_or(&[]);
    if rest.is_empty() {
        return None;
    }
    let (line, used) = match rest.iter().position(|c| *c == b'\n') {
        Some(i) => (&rest[..i], i + 1),
        None => (rest, rest.len()),
    };
    input.p += used;
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    Some(String::from_utf8_lossy(line).into_owned())
}

/// zombie.iso.IsoLot: the tile data of one 8x8 chunk across all levels, as read from a `.lotpack`.
/// `data` holds, per non-empty square, a count followed by that many indices into `LotHeader::tiles_used`.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoLot {
    pub max_level: i32,
    pub min_level: i32,
    pub wx: i32,
    pub wy: i32,
    /// Index: `x + y * 8 + (z - info.min_level) * 64`; -1 for an empty square. As in the pooled original, entries beyond
    /// the levels of the last loaded cell keep whatever an earlier load left there.
    pub offset_in_data: Vec<i32>,
    pub data: Vec<i32>,
}

impl Default for IsoLot {
    fn default() -> Self {
        IsoLot {
            max_level: 0,
            min_level: 0,
            wx: 0,
            wy: 0,
            offset_in_data: vec![0; 4096],
            data: Vec::new(),
        }
    }
}

impl IsoLot {
    /// zombie.iso.IsoLot.load: `lotpack` is the whole `world_<cX>_<cY>.lotpack`, `info` its cell header.
    /// On any error the original logs it and leaves the chunk empty; the error text is returned for the caller to log.
    pub fn load(
        &mut self,
        info: &super::lot_header::LotHeader,
        lotpack: &[u8],
        c_x: i32,
        c_y: i32,
        w_x: i32,
        w_y: i32,
    ) -> Option<String> {
        self.wx = w_x;
        self.wy = w_y;
        match self.load_inner(info, lotpack, c_x, c_y) {
            Ok(()) => None,
            Err(e) => {
                self.offset_in_data.fill(-1);
                self.data.clear();
                Some(e)
            }
        }
    }

    fn load_inner(
        &mut self,
        info: &super::lot_header::LotHeader,
        lotpack: &[u8],
        c_x: i32,
        c_y: i32,
    ) -> Result<(), String> {
        let mut input = RandomAccess::new(lotpack);
        let version = if input.read_bytes(4) == super::lot_header::LOTPACK_MAGIC {
            let v = read_int(&mut input)?;
            if !(0..=1).contains(&v) {
                return Err(format!("java.io.IOException: Unsupported version {v}"));
            }
            v
        } else {
            0
        };
        let mut skip = 0;
        let lwx = self.wx.wrapping_sub(c_x.wrapping_mul(32));
        let lwy = self.wy.wrapping_sub(c_y.wrapping_mul(32));
        let index = lwx.wrapping_mul(32).wrapping_add(lwy);
        let table = (if version >= 1 { 8i64 } else { 0 }) + 4 + index as i64 * 8;
        input
            .seek(usize::try_from(table).map_err(|_| "java.io.IOException: Negative seek offset")?);
        let pos = read_int(&mut input)?;
        input.seek(usize::try_from(pos).map_err(|_| "java.io.IOException: Negative seek offset")?);
        self.data.clear();
        let min_z = info.min_level.max(-32);
        let max_z = info.max_level.min(31);
        self.min_level = 0;
        let mut used_z = 0;
        for z in min_z..=max_z {
            for x in 0..8 {
                for y in 0..8 {
                    let square_xyz = x + y * 8 + (z - info.min_level) * 8 * 8;
                    *self
                        .offset_in_data
                        .get_mut(square_xyz as usize)
                        .ok_or("java.lang.ArrayIndexOutOfBoundsException")? = -1;
                    if skip > 0 {
                        skip -= 1;
                        continue;
                    }
                    let count = read_int(&mut input)?;
                    if count == -1 {
                        skip = read_int(&mut input)?;
                        if skip > 0 {
                            skip -= 1;
                            continue;
                        }
                    }
                    if count > 1 {
                        self.offset_in_data[square_xyz as usize] = self.data.len() as i32;
                        self.data.push(count - 1);
                        self.min_level = self.min_level.min(z);
                        used_z = used_z.max(z);
                        let _room = read_int(&mut input)?;
                        for _ in 1..count {
                            self.data.push(read_int(&mut input)?);
                        }
                    }
                }
            }
        }
        self.max_level = used_z + 1;
        Ok(())
    }
}
