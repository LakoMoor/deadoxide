//! java.nio.ByteBuffer reading (big-endian, the default byte order the game never changes) and the game's small
//! framing helpers built on it: `GameWindow.ReadString`, `zombie.util.io.BitHeader`, `zombie.core.utils.ByteBlock`.
//! Every read that would pass the limit is `Err`, where Java throws BufferUnderflowException.

const UNDERFLOW: &str = "java.nio.BufferUnderflowException";

/// java.nio.ByteBuffer over a byte slice, read-only.
#[derive(Clone, Debug)]
pub struct ByteBuffer<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> ByteBuffer<'a> {
    pub fn wrap(data: &'a [u8]) -> Self {
        ByteBuffer { data, position: 0 }
    }
    pub fn position(&self) -> usize {
        self.position
    }
    pub fn limit(&self) -> usize {
        self.data.len()
    }
    pub fn remaining(&self) -> usize {
        self.data.len() - self.position
    }
    pub fn array(&self) -> &'a [u8] {
        self.data
    }
    /// java.nio.Buffer.position(int): IllegalArgumentException past the limit.
    pub fn set_position(&mut self, p: usize) -> Result<(), String> {
        if p > self.data.len() {
            return Err(format!(
                "java.lang.IllegalArgumentException: newPosition > limit: ({p} > {})",
                self.data.len()
            ));
        }
        self.position = p;
        Ok(())
    }
    fn take<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let b = self
            .data
            .get(self.position..self.position + N)
            .ok_or(UNDERFLOW)?;
        self.position += N;
        let mut out = [0u8; N];
        out.copy_from_slice(b);
        Ok(out)
    }
    pub fn get_bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        let b = self
            .data
            .get(self.position..self.position + n)
            .ok_or(UNDERFLOW)?;
        self.position += n;
        Ok(b)
    }
    pub fn get(&mut self) -> Result<i8, String> {
        Ok(i8::from_be_bytes(self.take::<1>()?))
    }
    pub fn get_short(&mut self) -> Result<i16, String> {
        Ok(i16::from_be_bytes(self.take::<2>()?))
    }
    pub fn get_int(&mut self) -> Result<i32, String> {
        Ok(i32::from_be_bytes(self.take::<4>()?))
    }
    pub fn get_long(&mut self) -> Result<i64, String> {
        Ok(i64::from_be_bytes(self.take::<8>()?))
    }
    pub fn get_float(&mut self) -> Result<f32, String> {
        Ok(f32::from_bits(u32::from_be_bytes(self.take::<4>()?)))
    }
    pub fn get_double(&mut self) -> Result<f64, String> {
        Ok(f64::from_bits(u64::from_be_bytes(self.take::<8>()?)))
    }
}

/// zombie.GameWindow.ReadString (ByteBuffer) -> GameWindow.StringUTF.load: a short byte count, then that many bytes
/// decoded as UTF-8 with malformed input replaced (U+FFFD). A count of 0 or less is the empty string.
pub fn read_string(b: &mut ByteBuffer) -> Result<String, String> {
    let n = b.get_short()?;
    if n <= 0 {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(b.get_bytes(n as usize)?).into_owned())
}

/// zombie.util.io.BitHeader.HeaderSize
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderSize {
    Byte,
    Short,
    Integer,
    Long,
}

/// zombie.util.io.BitHeaderRead: a flag word read from the buffer.
#[derive(Clone, Copy, Debug)]
pub struct BitHeaderRead {
    header: i64,
    limit: i64,
}

impl BitHeaderRead {
    /// zombie.util.io.BitHeader.allocRead
    pub fn alloc_read(size: HeaderSize, b: &mut ByteBuffer) -> Result<Self, String> {
        Ok(match size {
            HeaderSize::Byte => BitHeaderRead {
                header: i64::from(b.get()?),
                limit: 64,
            },
            HeaderSize::Short => BitHeaderRead {
                header: i64::from(b.get_short()?),
                limit: 16384,
            },
            HeaderSize::Integer => BitHeaderRead {
                header: i64::from(b.get_int()?),
                limit: 1 << 30,
            },
            HeaderSize::Long => BitHeaderRead {
                header: b.get_long()?,
                limit: 1 << 62,
            },
        })
    }

    /// zombie.core.utils.Bits.hasFlags -> checkFlags(..., ContainsAll); flags above the size's limit throw.
    pub fn has_flags(&self, flags: i64) -> Result<bool, String> {
        if flags < 0 || flags > self.limit {
            return Err(format!(
                "java.lang.RuntimeException: Cannot check for flags, exceeding byte bounds or negative number flags. ({flags})"
            ));
        }
        Ok(self.header & flags == flags)
    }

    /// zombie.util.io.BitHeader.BitHeaderBase.equals
    pub fn equals(&self, flags: i64) -> bool {
        self.header == flags
    }
}

/// zombie.core.utils.ByteBlock in load mode: an int length, then that many bytes.
#[derive(Clone, Copy, Debug)]
pub struct ByteBlock {
    start: usize,
    length: usize,
}

impl ByteBlock {
    /// zombie.core.utils.ByteBlock.Start (Load)
    pub fn start(b: &mut ByteBuffer) -> Result<Self, String> {
        let length = b.get_int()?;
        if length < 0 {
            return Err("java.io.IOException: DataBlock possible corruption.".into());
        }
        Ok(ByteBlock {
            start: b.position(),
            length: length as usize,
        })
    }

    /// zombie.core.utils.ByteBlock.End (Load): the block must end exactly here.
    pub fn end(self, b: &ByteBuffer) -> Result<(), String> {
        if b.position() != self.start + self.length {
            return Err("java.io.IOException: DataBlock size mismatch during load.".into());
        }
        Ok(())
    }

    /// zombie.core.utils.ByteBlock.SkipAndEnd
    pub fn skip_and_end(self, b: &mut ByteBuffer) -> Result<(), String> {
        b.set_position(self.start + self.length)
    }
}

/// zombie.core.utils.Bits.unpackByteToFloatUnit
pub fn unpack_byte_to_float_unit(b: i8) -> f32 {
    (i32::from(b) - -128) as f32 / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_big_endian_and_utf8_strings() {
        let data = [0, 3, b'a', 0xC3, 0xA9, 0xFF, 0xFE, 0x40, 0x49, 0x0F, 0xDB];
        let mut b = ByteBuffer::wrap(&data);
        assert_eq!(read_string(&mut b).as_deref(), Ok("a\u{e9}"));
        assert_eq!(b.get_short(), Ok(-2));
        assert_eq!(b.get_float(), Ok(std::f32::consts::PI));
        assert!(b.get().is_err());
    }

    #[test]
    fn byte_header_rejects_flags_above_64() {
        let data = [0x41u8];
        let h = BitHeaderRead::alloc_read(HeaderSize::Byte, &mut ByteBuffer::wrap(&data));
        let h = h.expect("one byte");
        assert_eq!(h.has_flags(1), Ok(true));
        assert_eq!(h.has_flags(64), Ok(true));
        assert_eq!(h.has_flags(2), Ok(false));
        assert!(h.has_flags(128).is_err());
    }
}
