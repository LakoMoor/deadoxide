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
