//! se.krka.kahlua.j2se.KahluaTableImpl: the save form of a Lua table (object and square `modData`).
use crate::java_nio::{read_string, ByteBuffer};

/// A Lua value as a saved table can hold it.
#[derive(Clone, Debug, PartialEq)]
pub enum LuaValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Table(KahluaTable),
}

/// se.krka.kahlua.j2se.KahluaTableImpl: entries in first-insertion order (`rawset` of an existing key replaces the
/// value in place). The original iterates in its hash order, which this does not reproduce.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KahluaTable {
    pub entries: Vec<(LuaValue, LuaValue)>,
}

/// Raw key equality: strings and booleans by value, numbers as `Double.equals` (bit pattern), tables by identity
/// (two loaded tables are never the same key).
fn same_key(a: &LuaValue, b: &LuaValue) -> bool {
    match (a, b) {
        (LuaValue::String(x), LuaValue::String(y)) => x == y,
        (LuaValue::Number(x), LuaValue::Number(y)) => x.to_bits() == y.to_bits(),
        (LuaValue::Boolean(x), LuaValue::Boolean(y)) => x == y,
        _ => false,
    }
}

impl KahluaTable {
    /// se.krka.kahlua.j2se.KahluaTableImpl.rawset for a non-nil value.
    pub fn rawset(&mut self, key: LuaValue, value: LuaValue) {
        match self.entries.iter_mut().find(|(k, _)| same_key(k, &key)) {
            Some(e) => e.1 = value,
            None => self.entries.push((key, value)),
        }
    }

    /// se.krka.kahlua.j2se.KahluaTableImpl.load (ByteBuffer, int): a count, then typed keys and values (world
    /// version 25 and later; older saves stored string keys only).
    pub fn load(b: &mut ByteBuffer, world_version: i32) -> Result<Self, String> {
        let count = b.get_int()?;
        let mut t = KahluaTable::default();
        for _ in 0..count {
            if world_version >= 25 {
                let key_byte = b.get()?;
                let key = Self::load_value(b, world_version, key_byte)?;
                let value_byte = b.get()?;
                let value = Self::load_value(b, world_version, value_byte)?;
                t.rawset(key, value);
            } else {
                let value_byte = b.get()?;
                let key = LuaValue::String(read_string(b)?);
                let value = Self::load_value(b, world_version, value_byte)?;
                t.rawset(key, value);
            }
        }
        Ok(t)
    }

    /// se.krka.kahlua.j2se.KahluaTableImpl.load (ByteBuffer, int, byte)
    fn load_value(b: &mut ByteBuffer, world_version: i32, sbyt: i8) -> Result<LuaValue, String> {
        Ok(match sbyt {
            0 => LuaValue::String(read_string(b)?),
            1 => LuaValue::Number(b.get_double()?),
            3 => LuaValue::Boolean(b.get()? != 0),
            2 => LuaValue::Table(Self::load(b, world_version)?),
            _ => {
                return Err(format!(
                    "java.lang.RuntimeException: invalid lua table type {sbyt}"
                ))
            }
        })
    }
}
