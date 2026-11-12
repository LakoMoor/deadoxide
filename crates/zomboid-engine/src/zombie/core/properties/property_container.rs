use std::collections::BTreeMap;

use crate::java;
use crate::zombie::core::math::pz_math;
use crate::zombie::core::tile_property_alias_map::TilePropertyAliasMap;
use crate::zombie::iso::iso_directions::IsoDirections;
use crate::zombie::iso::sprite_details::iso_flag_type::IsoFlagType;
use crate::zombie::util::string_utils;

/// Values the original caches in `initSurface`.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Surface {
    pub surface: i8,
    pub is_surface_offset: bool,
    pub is_table: bool,
    pub is_table_top: bool,
    pub stack_replace_tile_offset: i16,
    pub item_height: i8,
    pub sloped_surface_direction: Option<IsoDirections>,
    pub sloped_surface_height_min: i8,
    pub sloped_surface_height_max: i8,
}

/// zombie.core.properties.PropertyContainer: 128 flag bits plus a map property-id -> value-id.
/// The Trove map of the original is a sorted map here; nothing observable depends on its iteration order.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct PropertyContainer {
    sprite_flags1: u64,
    sprite_flags2: u64,
    map: BTreeMap<u16, u16>,
    /// The original's `keyArray != null`. While set, the array mirrors the map's keys.
    has_key_array: bool,
}

impl PropertyContainer {
    pub fn new() -> Self {
        Self::default()
    }

    /// zombie.core.properties.PropertyContainer.CreateKeySet
    pub fn create_key_set(&mut self) {
        self.has_key_array = !self.map.is_empty();
    }

    /// zombie.core.properties.PropertyContainer.AddProperties: values are copied only from a container whose
    /// key array exists (`CreateKeySet` was called while it held values); flags are always merged.
    pub fn add_properties(&mut self, other: &PropertyContainer) {
        if other.has_key_array {
            for (k, v) in &other.map {
                self.map.insert(*k, *v);
            }
        }
        self.sprite_flags1 |= other.sprite_flags1;
        self.sprite_flags2 |= other.sprite_flags2;
    }

    /// zombie.core.properties.PropertyContainer.Clear
    pub fn clear(&mut self) {
        self.sprite_flags1 = 0;
        self.sprite_flags2 = 0;
        self.map.clear();
        self.has_key_array = false;
    }

    /// zombie.core.properties.PropertyContainer.has (IsoFlagType)
    pub fn has(&self, flag: IsoFlagType) -> bool {
        let flags = if flag.index() < 64 {
            self.sprite_flags1
        } else {
            self.sprite_flags2
        };
        flags & (1u64 << (flag.index() & 63)) != 0
    }

    /// zombie.core.properties.PropertyContainer.set (String, String, boolean): a name that is an `IsoFlagType`
    /// sets the flag and stores no value when `check_iso_flag_type`; an unregistered name is dropped.
    pub fn set_str(
        &mut self,
        alias: &TilePropertyAliasMap,
        prop_name: &str,
        prop_value: &str,
        check_iso_flag_type: bool,
    ) {
        if check_iso_flag_type {
            let e = IsoFlagType::from_string(prop_name);
            if e != IsoFlagType::MAX {
                self.set(e);
                return;
            }
        }
        if let Some(p) = alias.get_id_from_property_name(prop_name) {
            let v = alias.get_id_from_property_value(p, prop_value);
            self.map.insert(p as u16, v as u16);
        }
    }

    /// zombie.core.properties.PropertyContainer.set (IsoFlagType)
    pub fn set(&mut self, flag: IsoFlagType) {
        if flag.index() / 64 == 0 {
            self.sprite_flags1 |= 1u64 << (flag.index() % 64);
        } else {
            self.sprite_flags2 |= 1u64 << (flag.index() % 64);
        }
    }

    /// zombie.core.properties.PropertyContainer.unset (String)
    pub fn unset_str(&mut self, alias: &TilePropertyAliasMap, prop_name: &str) {
        if let Some(p) = alias.get_id_from_property_name(prop_name) {
            self.map.remove(&(p as u16));
        }
    }

    /// zombie.core.properties.PropertyContainer.unset (IsoFlagType)
    pub fn unset(&mut self, flag: IsoFlagType) {
        if flag.index() / 64 == 0 {
            self.sprite_flags1 &= !(1u64 << (flag.index() % 64));
        } else {
            self.sprite_flags2 &= !(1u64 << (flag.index() % 64));
        }
    }

    /// zombie.core.properties.PropertyContainer.get (String): `None` is Java null.
    pub fn get<'a>(&self, alias: &'a TilePropertyAliasMap, name: &str) -> Option<&'a str> {
        let p = alias.get_id_from_property_name(name)?;
        let v = self.map.get(&(p as u16))?;
        Some(alias.get_property_value_string(p, *v as usize))
    }

    /// zombie.core.properties.PropertyContainer.propertyEquals
    pub fn property_equals(&self, alias: &TilePropertyAliasMap, name: &str, value: &str) -> bool {
        string_utils::equals_ignore_case(self.get(alias, name), Some(value))
    }

    /// zombie.core.properties.PropertyContainer.has (String)
    pub fn has_str(&self, alias: &TilePropertyAliasMap, name: &str) -> bool {
        alias
            .get_id_from_property_name(name)
            .is_some_and(|p| self.map.contains_key(&(p as u16)))
    }

    /// zombie.core.properties.PropertyContainer.getFlagsList
    pub fn get_flags_list(&self) -> Vec<IsoFlagType> {
        let mut ret = Vec::new();
        for i in 0..64 {
            if self.sprite_flags1 & (1u64 << i) != 0 {
                ret.push(IsoFlagType::from_index(i));
            }
        }
        for i in 0..64 {
            // Bits past MAX cannot be set through `set`; the original would throw on them.
            if self.sprite_flags2 & (1u64 << i) != 0 {
                ret.push(IsoFlagType::from_index(64 + i));
            }
        }
        ret
    }

    /// zombie.core.properties.PropertyContainer.getPropertyNames: sorted, as the original sorts.
    pub fn get_property_names<'a>(&self, alias: &'a TilePropertyAliasMap) -> Vec<&'a str> {
        let mut list: Vec<&str> = self
            .map
            .keys()
            .map(|k| alias.properties[*k as usize].property_name.as_str())
            .collect();
        list.sort_unstable();
        list
    }

    /// zombie.core.properties.PropertyContainer.initSurface: computed on request instead of cached.
    /// An invalid `SlopedSurfaceDirection` yields `None` here; the original throws from `IsoDirections.fromString`.
    pub fn init_surface(&self, alias: &TilePropertyAliasMap) -> Surface {
        let mut s = Surface::default();
        for (k, v) in &self.map {
            let p = &alias.properties[*k as usize];
            // The original indexes possibleValues directly (an empty list would throw there).
            let val = p.possible_values.get(*v as usize).map(String::as_str);
            let in_byte = |x: i32| (0..=127).contains(&x);
            match p.property_name.as_str() {
                "Surface" => {
                    if let Some(pixels) = val.and_then(java::parse_int).filter(|x| in_byte(*x)) {
                        s.surface = pixels as i8;
                    }
                }
                "IsSurfaceOffset" => s.is_surface_offset = true,
                "IsTable" => s.is_table = true,
                "IsTableTop" => s.is_table_top = true,
                "StackReplaceTileOffset" => {
                    if let Some(x) = val.and_then(java::parse_int) {
                        s.stack_replace_tile_offset = x as i16;
                    }
                }
                "ItemHeight" => {
                    if let Some(pixels) = val.and_then(java::parse_int).filter(|x| in_byte(*x)) {
                        s.item_height = pixels as i8;
                    }
                }
                "SlopedSurfaceDirection" => {
                    s.sloped_surface_direction = val.and_then(IsoDirections::from_string)
                }
                "SlopedSurfaceHeightMin" => {
                    s.sloped_surface_height_min =
                        pz_math::clamp_i32(pz_math::try_parse_int(val, 0), 0, 100) as i8
                }
                "SlopedSurfaceHeightMax" => {
                    s.sloped_surface_height_max =
                        pz_math::clamp_i32(pz_math::try_parse_int(val, 0), 0, 100) as i8
                }
                _ => {}
            }
        }
        s
    }

    /// zombie.core.properties.PropertyContainer.getSurface
    pub fn get_surface(&self, alias: &TilePropertyAliasMap) -> i32 {
        self.init_surface(alias).surface as i32
    }
    /// zombie.core.properties.PropertyContainer.isSurfaceOffset
    pub fn is_surface_offset(&self, alias: &TilePropertyAliasMap) -> bool {
        self.init_surface(alias).is_surface_offset
    }
    /// zombie.core.properties.PropertyContainer.isTable
    pub fn is_table(&self, alias: &TilePropertyAliasMap) -> bool {
        self.init_surface(alias).is_table
    }
    /// zombie.core.properties.PropertyContainer.isTableTop
    pub fn is_table_top(&self, alias: &TilePropertyAliasMap) -> bool {
        self.init_surface(alias).is_table_top
    }
    /// zombie.core.properties.PropertyContainer.getStackReplaceTileOffset
    pub fn get_stack_replace_tile_offset(&self, alias: &TilePropertyAliasMap) -> i32 {
        self.init_surface(alias).stack_replace_tile_offset as i32
    }
    /// zombie.core.properties.PropertyContainer.getItemHeight
    pub fn get_item_height(&self, alias: &TilePropertyAliasMap) -> i32 {
        self.init_surface(alias).item_height as i32
    }
    /// zombie.core.properties.PropertyContainer.getSlopedSurfaceDirection
    pub fn get_sloped_surface_direction(
        &self,
        alias: &TilePropertyAliasMap,
    ) -> Option<IsoDirections> {
        self.init_surface(alias).sloped_surface_direction
    }
    /// zombie.core.properties.PropertyContainer.getSlopedSurfaceHeightMin
    pub fn get_sloped_surface_height_min(&self, alias: &TilePropertyAliasMap) -> i32 {
        self.init_surface(alias).sloped_surface_height_min as i32
    }
    /// zombie.core.properties.PropertyContainer.getSlopedSurfaceHeightMax
    pub fn get_sloped_surface_height_max(&self, alias: &TilePropertyAliasMap) -> i32 {
        self.init_surface(alias).sloped_surface_height_max as i32
    }
}
