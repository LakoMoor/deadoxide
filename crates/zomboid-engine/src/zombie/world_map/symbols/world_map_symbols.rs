//! zombie.worldMap.symbols.WorldMapSymbols: the symbols (notes and stamps) a player draws on a map item, as a save stores them.
use crate::java_nio::{read_string, ByteBuffer};
use crate::zombie::core::math::pz_math;

/// zombie.worldMap.symbols.WorldMapBaseSymbol, the saved fields.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldMapBaseSymbol {
    pub x: f32,
    pub y: f32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub scale: f32,
    /// `setRotation`: wrapped into [0, 360).
    pub rotation: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub collide: bool,
    pub match_perspective: bool,
    pub apply_zoom: bool,
    /// `None` keeps the field initialiser.
    pub min_zoom: Option<f32>,
    pub max_zoom: Option<f32>,
}

/// zombie.worldMap.symbols.WorldMapSymbols.WorldMapSymbolType: the text and texture symbol classes.
#[derive(Clone, Debug, PartialEq)]
pub enum WorldMapSymbol {
    /// zombie.worldMap.symbols.WorldMapTextSymbol
    Text {
        base: WorldMapBaseSymbol,
        text: String,
        translated: bool,
        /// `None` before symbols version 2 (the field initialiser stays).
        layer_id: Option<String>,
    },
    /// zombie.worldMap.symbols.WorldMapTextureSymbol
    Texture {
        base: WorldMapBaseSymbol,
        symbol_id: String,
    },
}

/// zombie.worldMap.symbols.WorldMapSymbols, the saved part.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldMapSymbols {
    pub symbols: Vec<WorldMapSymbol>,
}

/// zombie.worldMap.symbols.SymbolSaveData: the font names that text symbols refer to by index.
struct SymbolSaveData {
    symbols_version: i16,
    index_to_font_name: Vec<String>,
}

impl SymbolSaveData {
    /// zombie.worldMap.symbols.SymbolSaveData.load: an unsigned byte count, then the names.
    fn load(&mut self, b: &mut ByteBuffer) -> Result<(), String> {
        let num_fonts = i32::from(b.get()?) & 255;
        for _ in 0..num_fonts {
            self.index_to_font_name.push(read_string(b)?);
        }
        Ok(())
    }
}

impl WorldMapBaseSymbol {
    /// zombie.worldMap.symbols.WorldMapBaseSymbol.load
    fn load(b: &mut ByteBuffer, save_data: &SymbolSaveData) -> Result<Self, String> {
        let x = b.get_float()?;
        let y = b.get_float()?;
        let anchor_x = b.get_float()?;
        let anchor_y = b.get_float()?;
        let scale = b.get_float()?;
        let rotation = if save_data.symbols_version >= 2 {
            pz_math::wrap(b.get_float()?, 0.0, 360.0)
        } else {
            0.0
        };
        let unit = |v: i8| (i32::from(v) & 255) as f32 / 255.0;
        let r = unit(b.get()?);
        let g = unit(b.get()?);
        let bl = unit(b.get()?);
        let a = unit(b.get()?);
        let collide = b.get()? != 0;
        let mut s = WorldMapBaseSymbol {
            x,
            y,
            anchor_x,
            anchor_y,
            scale,
            rotation,
            r,
            g,
            b: bl,
            a,
            collide,
            match_perspective: false,
            // field initialiser
            apply_zoom: true,
            min_zoom: None,
            max_zoom: None,
        };
        if save_data.symbols_version >= 2 {
            let flags = i32::from(b.get()?) & 255;
            s.match_perspective = flags & 1 != 0;
            s.apply_zoom = flags & 2 != 0;
            if flags & 4 != 0 {
                s.min_zoom = Some(pz_math::clamp(b.get_float()?, 0.0, 24.0));
            }
            if flags & 8 != 0 {
                s.max_zoom = Some(pz_math::clamp(b.get_float()?, 0.0, 24.0));
            }
        }
        Ok(s)
    }
}

impl WorldMapSymbols {
    /// zombie.worldMap.symbols.WorldMapSymbols.load: a version (1 or 2), the font table (version 2), then the
    /// symbols, each a type byte (0 text, 1 texture) and its fields.
    pub fn load(b: &mut ByteBuffer) -> Result<Self, String> {
        let symbols_version = b.get_short()?;
        if !(1..=2).contains(&symbols_version) {
            return Err(format!(
                "java.io.IOException: unknown map symbols version {symbols_version}"
            ));
        }
        let mut save_data = SymbolSaveData {
            symbols_version,
            index_to_font_name: Vec::new(),
        };
        if symbols_version >= 2 {
            save_data.load(b)?;
        }
        let symbol_count = b.get_int()?;
        let mut out = WorldMapSymbols::default();
        for _ in 0..symbol_count.max(0) {
            let symbol_type = b.get()?;
            out.symbols.push(match symbol_type {
                // WorldMapTextSymbol.load
                0 => {
                    let base = WorldMapBaseSymbol::load(b, &save_data)?;
                    let text = read_string(b)?;
                    let translated = b.get()? != 0;
                    let layer_id = if symbols_version >= 2 {
                        let font_index = (i32::from(b.get()?) & 255) as usize;
                        Some(
                            save_data
                                .index_to_font_name
                                .get(font_index)
                                .cloned()
                                .unwrap_or_else(|| "text-note".to_string()),
                        )
                    } else {
                        None
                    };
                    WorldMapSymbol::Text {
                        base,
                        text,
                        translated,
                        layer_id,
                    }
                }
                // WorldMapTextureSymbol.load
                1 => {
                    let base = WorldMapBaseSymbol::load(b, &save_data)?;
                    WorldMapSymbol::Texture {
                        base,
                        symbol_id: read_string(b)?,
                    }
                }
                t => return Err(format!("java.io.IOException: unknown map symbol type {t}")),
            });
        }
        Ok(out)
    }
}
