//! Texture packs (`media/texturepacks/*.pack`). Layout from bytes + TexturePackDevice control flow (build 25485521):
//! Optional header: "PZPK" + u32 version (0 or 1); absent = version 0. v0: raw PNG then 0xDEADBEEF marker; v1: u32 png_len then PNG.
//! u32 page_count; per page: str name, u32 entry_count, u32 has_alpha,
//! entry_count x { str name, i32 x,y,w,h, offx,offy, origw,origh }, then either a raw PNG (walked to IEND) or u32 0xDEADBEEF, u32 png_len, png bytes.
//! str = u32 len + bytes. All integers little-endian.
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct SubTexture {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub off_x: i32,
    pub off_y: i32,
    pub orig_w: i32,
    pub orig_h: i32,
}

#[derive(Debug)]
pub struct Page {
    pub name: String,
    pub has_alpha: bool,
    pub entries: Vec<SubTexture>,
    pub png: Range<usize>,
}

#[derive(Debug)]
pub enum PackError {
    Truncated(&'static str),
    BadMagic(usize, u32),
    NotPng(usize),
    Trailing(usize),
}

struct R<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> R<'a> {
    fn u32(&mut self, w: &'static str) -> Result<u32, PackError> {
        let s = self
            .b
            .get(self.p..self.p + 4)
            .ok_or(PackError::Truncated(w))?;
        self.p += 4;
        Ok(u32::from_le_bytes(s.try_into().unwrap()))
    }
    fn str(&mut self, w: &'static str) -> Result<String, PackError> {
        let n = self.u32(w)? as usize;
        let s = self
            .b
            .get(self.p..self.p + n)
            .ok_or(PackError::Truncated(w))?;
        self.p += n;
        Ok(String::from_utf8_lossy(s).into_owned())
    }
}

/// End offset of a PNG that starts at `start` (walks chunks to IEND).
fn png_end(b: &[u8], start: usize) -> Result<usize, PackError> {
    let mut p = start + 8;
    loop {
        let h = b.get(p..p + 8).ok_or(PackError::Truncated("png chunk"))?;
        let len = u32::from_be_bytes(h[..4].try_into().unwrap()) as usize;
        p += 12 + len;
        if &h[4..8] == b"IEND" {
            return Ok(p);
        }
    }
}

pub fn parse_pack(b: &[u8]) -> Result<Vec<Page>, PackError> {
    let mut r = R { b, p: 0 };
    let mut version = 0;
    if b.starts_with(b"PZPK") {
        r.p = 4;
        version = r.u32("version")?;
        if version > 1 {
            return Err(PackError::Truncated("unsupported PZPK version"));
        }
    }
    let pages = r.u32("page count")?;
    let mut out = Vec::new();
    for _ in 0..pages {
        let name = r.str("page name")?;
        let n = r.u32("entry count")?;
        let has_alpha = r.u32("alpha")? != 0;
        let mut entries = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let name = r.str("entry name")?;
            let mut v = [0i32; 8];
            for x in &mut v {
                *x = r.u32("entry field")? as i32;
            }
            entries.push(SubTexture {
                name,
                x: v[0],
                y: v[1],
                w: v[2],
                h: v[3],
                off_x: v[4],
                off_y: v[5],
                orig_w: v[6],
                orig_h: v[7],
            });
        }
        let (start, end) = if version >= 1 {
            let len = r.u32("png len")? as usize;
            (r.p, r.p + len)
        } else {
            let s = r.p;
            (s, png_end(b, s)?)
        };
        let png = b.get(start..end).ok_or(PackError::Truncated("png"))?;
        if !png.starts_with(&[0x89, b'P', b'N', b'G']) {
            return Err(PackError::NotPng(start));
        }
        r.p = end;
        if b.get(r.p..r.p + 4) == Some(&0xDEAD_BEEFu32.to_le_bytes()[..]) {
            r.p += 4;
        }
        out.push(Page {
            name,
            has_alpha,
            entries,
            png: start..end,
        });
    }
    if r.p != b.len() {
        return Err(PackError::Trailing(b.len() - r.p));
    }
    Ok(out)
}
