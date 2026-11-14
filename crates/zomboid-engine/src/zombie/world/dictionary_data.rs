//! zombie.world.DictionaryData: a save's `WorldDictionary.bin`, the id tables chunk saves refer to (item and entity
//! registry ids, object-name ids, sprite-name ids) plus the string and script registers.
use std::collections::{BTreeMap, HashMap};

use crate::java_nio::{read_string, BitHeaderRead, ByteBlock, ByteBuffer, HeaderSize};
use crate::zombie::iso::sprite::iso_sprite_manager::IsoSpriteManager;

/// zombie.world.DictionaryInfo (ItemInfo and EntityInfo share it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryInfo {
    pub registry_id: i16,
    pub module_name: String,
    pub name: String,
    pub full_type: String,
    pub mod_id: String,
    pub is_modded: bool,
    pub exists_as_vanilla: bool,
    pub obsolete: bool,
    pub removed: bool,
    pub mod_overrides: Option<Vec<String>>,
}

/// zombie.world.DictionaryStringInfo
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryStringInfo {
    pub registry_id: i16,
    pub string: String,
    pub is_loaded: bool,
}

/// zombie.world.DictionaryScriptInfo
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryScriptInfo {
    pub registry_id: i16,
    pub version: i64,
    pub name: String,
    pub is_loaded: bool,
}

/// zombie.world.StringDictionary.StringRegister / zombie.world.ScriptsDictionary.ScriptRegister: entries by id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Register<T> {
    pub name: &'static str,
    pub next_id: i16,
    pub by_id: BTreeMap<i16, T>,
}

/// zombie.world.StringDictionary.registers (in declaration order).
pub const STRING_REGISTERS: [&str; 1] = ["Generic"];
/// zombie.world.ScriptsDictionary.registers (in declaration order).
pub const SCRIPT_REGISTERS: [&str; 3] = [
    "SpriteConfigs",
    "SpriteOverlayConfigs",
    "ContextMenuConfigs",
];

/// zombie.world.DictionaryData
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryData {
    pub version: i32,
    pub next_info_id: i16,
    pub next_object_name_id: i8,
    pub next_sprite_name_id: i32,
    pub mod_ids: Vec<String>,
    pub modules: Vec<String>,
    /// itemIdToInfoMap
    pub items: BTreeMap<i16, DictionaryInfo>,
    /// itemTypeToInfoMap, as full type -> registry id
    pub item_types: HashMap<String, i16>,
    pub entities: BTreeMap<i16, DictionaryInfo>,
    pub entity_types: HashMap<String, i16>,
    /// objectIdToNameMap
    pub object_names: BTreeMap<i8, String>,
    pub object_ids: HashMap<String, i8>,
    /// spriteIdToNameMap
    pub sprite_names: BTreeMap<i32, String>,
    pub sprite_ids: HashMap<String, i32>,
    pub string_registers: Vec<Register<DictionaryStringInfo>>,
    pub script_registers: Vec<Register<DictionaryScriptInfo>>,
}

/// `modules.get(modules.size() > 127 ? bb.getShort() : bb.get())`
fn index_into<'a>(list: &'a [String], b: &mut ByteBuffer) -> Result<&'a String, String> {
    let i = if list.len() > 127 {
        i32::from(b.get_short()?)
    } else {
        i32::from(b.get()?)
    };
    usize::try_from(i)
        .ok()
        .and_then(|i| list.get(i))
        .ok_or_else(|| {
            format!(
                "java.lang.IndexOutOfBoundsException: Index {i} out of bounds for length {}",
                list.len()
            )
        })
}

impl DictionaryInfo {
    /// zombie.world.DictionaryInfo.load
    fn load(b: &mut ByteBuffer, mod_ids: &[String], modules: &[String]) -> Result<Self, String> {
        let registry_id = b.get_short()?;
        let module_name = index_into(modules, b)?.clone();
        let name = read_string(b)?;
        let full_type = format!("{module_name}.{name}");
        let bits = i32::from(b.get()?);
        let has = |f: i32| bits & f == f;
        let (mod_id, is_modded) = if has(1) {
            (index_into(mod_ids, b)?.clone(), true)
        } else {
            ("pz-vanilla".to_string(), false)
        };
        let mut mod_overrides = None;
        if has(16) {
            let mut list = Vec::new();
            if !has(32) {
                list.push(index_into(mod_ids, b)?.clone());
            } else {
                let count = b.get()?;
                for _ in 0..count {
                    list.push(index_into(mod_ids, b)?.clone());
                }
            }
            mod_overrides = Some(list);
        }
        Ok(DictionaryInfo {
            registry_id,
            module_name,
            name,
            full_type,
            mod_id,
            is_modded,
            exists_as_vanilla: has(2),
            obsolete: has(4),
            removed: has(8),
            mod_overrides,
        })
    }
}

impl DictionaryData {
    /// zombie.world.DictionaryData.loadFromByteBuffer (with StringDictionary and ScriptsDictionary.loadFromByteBuffer):
    /// HashMap `put` semantics, so a repeated id or name keeps the last entry; an unknown register is skipped.
    pub fn load_from_byte_buffer(data: &[u8]) -> Result<Self, String> {
        let mut b = ByteBuffer::wrap(data);
        let version = b.get_int()?;
        let next_info_id = b.get_short()?;
        let next_object_name_id = b.get()?;
        let next_sprite_name_id = b.get_int()?;
        let strings = |b: &mut ByteBuffer| -> Result<Vec<String>, String> {
            let n = b.get_int()?;
            (0..n).map(|_| read_string(b)).collect()
        };
        let mod_ids = strings(&mut b)?;
        let modules = strings(&mut b)?;
        let mut d = DictionaryData {
            version,
            next_info_id,
            next_object_name_id,
            next_sprite_name_id,
            mod_ids,
            modules,
            items: BTreeMap::new(),
            item_types: HashMap::new(),
            entities: BTreeMap::new(),
            entity_types: HashMap::new(),
            object_names: BTreeMap::new(),
            object_ids: HashMap::new(),
            sprite_names: BTreeMap::new(),
            sprite_ids: HashMap::new(),
            string_registers: STRING_REGISTERS
                .iter()
                .map(|n| Register {
                    name: n,
                    next_id: 0,
                    by_id: BTreeMap::new(),
                })
                .collect(),
            script_registers: SCRIPT_REGISTERS
                .iter()
                .map(|n| Register {
                    name: n,
                    next_id: 0,
                    by_id: BTreeMap::new(),
                })
                .collect(),
        };
        for _ in 0..b.get_int()? {
            let info = DictionaryInfo::load(&mut b, &d.mod_ids, &d.modules)?;
            d.item_types
                .insert(info.full_type.clone(), info.registry_id);
            d.items.insert(info.registry_id, info);
        }
        for _ in 0..b.get_int()? {
            let info = DictionaryInfo::load(&mut b, &d.mod_ids, &d.modules)?;
            d.entity_types
                .insert(info.full_type.clone(), info.registry_id);
            d.entities.insert(info.registry_id, info);
        }
        for _ in 0..b.get_int()? {
            let id = b.get()?;
            let name = read_string(&mut b)?;
            d.object_names.insert(id, name.clone());
            d.object_ids.insert(name, id);
        }
        for _ in 0..b.get_int()? {
            let id = b.get_int()?;
            let name = read_string(&mut b)?;
            d.sprite_names.insert(id, name.clone());
            d.sprite_ids.insert(name, id);
        }
        // zombie.world.StringDictionary.loadFromByteBuffer -> StringRegister.load / loadEmpty
        for _ in 0..b.get_int()? {
            let register = read_string(&mut b)?;
            let block = ByteBlock::start(&mut b)?;
            let Some(r) = d.string_registers.iter_mut().find(|r| r.name == register) else {
                block.skip_and_end(&mut b)?;
                continue;
            };
            r.next_id = b.get_short()?;
            for _ in 0..b.get_int()? {
                // zombie.world.DictionaryStringInfo.load
                let header = BitHeaderRead::alloc_read(HeaderSize::Byte, &mut b)?;
                let registry_id = b.get_short()?;
                let s = read_string(&mut b)?;
                let string = if header.has_flags(1)? {
                    format!("Base.{s}")
                } else {
                    s
                };
                r.by_id.insert(
                    registry_id,
                    DictionaryStringInfo {
                        registry_id,
                        string,
                        is_loaded: header.has_flags(2)?,
                    },
                );
            }
            block.end(&b)?;
        }
        // zombie.world.ScriptsDictionary.loadFromByteBuffer -> ScriptRegister.load / loadEmpty
        for _ in 0..b.get_int()? {
            let register = read_string(&mut b)?;
            let block = ByteBlock::start(&mut b)?;
            let Some(r) = d.script_registers.iter_mut().find(|r| r.name == register) else {
                block.skip_and_end(&mut b)?;
                continue;
            };
            r.next_id = b.get_short()?;
            for _ in 0..b.get_int()? {
                // zombie.world.DictionaryScriptInfo.load
                let header = BitHeaderRead::alloc_read(HeaderSize::Byte, &mut b)?;
                let registry_id = b.get_short()?;
                let version = b.get_long()?;
                let s = read_string(&mut b)?;
                let name = if header.has_flags(1)? {
                    format!("Base.{s}")
                } else {
                    s
                };
                r.by_id.insert(
                    registry_id,
                    DictionaryScriptInfo {
                        registry_id,
                        version,
                        name,
                        is_loaded: header.has_flags(2)?,
                    },
                );
            }
            block.end(&b)?;
        }
        Ok(d)
    }

    /// zombie.world.WorldDictionary.getItemTypeFromID, for the loaded table.
    pub fn get_item_type_from_id(&self, id: i16) -> Option<&str> {
        self.items.get(&id).map(|i| i.full_type.as_str())
    }

    /// zombie.world.WorldDictionary.getObjectNameFromID, for the loaded table.
    pub fn get_object_name_from_id(&self, id: i8) -> Option<&str> {
        self.object_names.get(&id).map(String::as_str)
    }

    /// zombie.world.DictionaryData.getSpriteNameFromID: the table's name for a non-negative id, else the name of the
    /// tile sprite with that id.
    pub fn get_sprite_name_from_id<'a>(
        &'a self,
        id: i32,
        sprites: &'a IsoSpriteManager,
    ) -> Option<&'a str> {
        if id < 0 {
            return None;
        }
        if let Some(n) = self.sprite_names.get(&id) {
            return Some(n);
        }
        sprites
            .get_sprite_by_id(id)
            .and_then(|s| sprites.sprite(s).name.as_deref())
    }
}
