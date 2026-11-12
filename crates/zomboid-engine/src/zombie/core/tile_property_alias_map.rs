use std::collections::{BTreeMap, HashMap};

/// zombie.core.TilePropertyAliasMap.TileProperty
#[derive(Default, Debug)]
pub struct TileProperty {
    pub property_name: String,
    pub possible_values: Vec<String>,
    pub id_map: HashMap<String, usize>,
}

/// zombie.core.TilePropertyAliasMap: interns property names and their value strings as small integers.
/// The original is a process-wide singleton; here it is a value passed to `PropertyContainer`.
#[derive(Default, Debug)]
pub struct TilePropertyAliasMap {
    pub property_to_id: HashMap<String, usize>,
    pub properties: Vec<TileProperty>,
}

impl TilePropertyAliasMap {
    /// zombie.core.TilePropertyAliasMap.Generate: `entity_script_names` stands for
    /// `ScriptManager.instance.getAllGameEntities()` names (generateEntityProperties).
    /// The original walks a `HashMap`, so its numeric ids follow Java's hash order; ids are never stored or shown,
    /// so this port assigns them in key order instead.
    pub fn generate(
        &mut self,
        property_value_map: &BTreeMap<String, Vec<String>>,
        entity_script_names: &[String],
    ) {
        self.properties.clear();
        self.property_to_id.clear();
        for (k, v) in property_value_map {
            self.register(k, v);
        }
        self.register(
            super::properties::iso_property_type::ENTITY_SCRIPT_NAME,
            entity_script_names,
        );
    }

    /// zombie.core.TilePropertyAliasMap.register
    fn register(&mut self, prop_key: &str, possible_prop_values: &[String]) {
        assert!(self.properties.len() < 32767, "too many properties defined");
        assert!(
            possible_prop_values.len() <= 32767,
            "too many property values defined for {prop_key}"
        );
        // IsoPropertyType.lookupOrDefaultStr returns the same text for known and unknown names.
        self.property_to_id
            .insert(prop_key.to_string(), self.properties.len());
        let mut new_prop = TileProperty {
            property_name: prop_key.to_string(),
            possible_values: possible_prop_values.to_vec(),
            id_map: HashMap::new(),
        };
        for (i, v) in possible_prop_values.iter().enumerate() {
            new_prop.id_map.insert(v.clone(), i); // a repeated value keeps its last index, as HashMap.put does
        }
        self.properties.push(new_prop);
    }

    /// zombie.core.TilePropertyAliasMap.getIDFromPropertyName: `None` is the original's -1.
    pub fn get_id_from_property_name(&self, name: &str) -> Option<usize> {
        self.property_to_id.get(name).copied()
    }

    /// zombie.core.TilePropertyAliasMap.getIDFromPropertyValue: an unknown value silently becomes id 0.
    pub fn get_id_from_property_value(&self, property: usize, value: &str) -> usize {
        let p = &self.properties[property];
        if p.possible_values.is_empty() {
            0
        } else {
            p.id_map.get(value).copied().unwrap_or(0)
        }
    }

    /// zombie.core.TilePropertyAliasMap.getPropertyValueString
    pub fn get_property_value_string(&self, property: usize, value: usize) -> &str {
        let p = &self.properties[property];
        if p.possible_values.is_empty() {
            ""
        } else {
            &p.possible_values[value]
        }
    }
}
