//! Builds one RGBA atlas from the sub-textures a scene needs (shelf packing). Prefers 2x packs.
use crate::pack::{parse_pack, SubTexture};
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub layer: u32,
    pub ax: u32,
    pub ay: u32,
    pub w: i32,
    pub h: i32,
    pub off_x: i32,
    pub off_y: i32,
    pub orig_w: i32,
    pub orig_h: i32,
}

pub struct Atlas {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub layers: u32,
    pub sprites: HashMap<String, Sprite>,
}

pub fn decode_png(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut rd = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; rd.output_buffer_size()];
    let info = rd.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let b = &buf[..info.buffer_size()];
    let px = match info.color_type {
        png::ColorType::Rgba => b.to_vec(),
        png::ColorType::Rgb => b.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => {
            b.chunks(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect()
        }
        png::ColorType::Grayscale => b.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        c => return Err(format!("unsupported png color {c:?}")),
    };
    Ok((px, info.width, info.height))
}

pub fn build_atlas(
    root: &Path,
    wanted: &HashSet<String>,
    atlas_width: u32,
    max_height: u32,
) -> Result<Atlas, String> {
    let mut files: Vec<_> = std::fs::read_dir(root.join("media/texturepacks"))
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("pack"))
        .collect();
    files.sort_by_key(|p| {
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        (!n.contains("2x"), n)
    });
    let found: HashMap<String, (Vec<u8>, u32, SubTexture)> = HashMap::new(); // name -> (page rgba shared? copy rect later)
    let mut pages: Vec<(Vec<u8>, u32)> = vec![];
    let mut sel: Vec<(String, usize, SubTexture)> = vec![];
    for p in files {
        let b = std::fs::read(&p).map_err(|e| e.to_string())?;
        for pg in parse_pack(&b).map_err(|e| format!("{}: {e:?}", p.display()))? {
            let hits: Vec<_> = pg
                .entries
                .iter()
                .filter(|e| {
                    wanted.contains(&e.name)
                        && !sel.iter().any(|s| s.0 == e.name)
                        && !found.contains_key(&e.name)
                })
                .cloned()
                .collect();
            if hits.is_empty() {
                continue;
            }
            let (px, w, _h) = decode_png(&b[pg.png.clone()])?;
            pages.push((px, w));
            for e in hits {
                sel.push((e.name.clone(), pages.len() - 1, e));
            }
        }
    }
    // reserved solid-colour block used for the player marker (not from the game)
    pages.push(([255u8, 40, 40, 255].repeat(8 * 8), 8));
    sel.push((
        "__player".to_string(),
        pages.len() - 1,
        SubTexture {
            name: "__player".into(),
            x: 0,
            y: 0,
            w: 8,
            h: 8,
            off_x: 0,
            off_y: 0,
            orig_w: 8,
            orig_h: 8,
        },
    ));
    pages.push(([40u8, 200, 60, 255].repeat(8 * 8), 8));
    sel.push((
        "__zombie".to_string(),
        pages.len() - 1,
        SubTexture {
            name: "__zombie".into(),
            x: 0,
            y: 0,
            w: 8,
            h: 8,
            off_x: 0,
            off_y: 0,
            orig_w: 8,
            orig_h: 8,
        },
    ));
    // shelf pack, tallest first
    sel.sort_by_key(|s| std::cmp::Reverse(s.2.h));
    let (mut x, mut y, mut row_h, mut layer) = (0u32, 0u32, 0u32, 0u32);
    let mut place = vec![];
    let mut layer_h = 1u32;
    for (_, _, e) in &sel {
        let (w, h) = (e.w as u32 + 1, e.h as u32 + 1);
        if x + w > atlas_width {
            x = 0;
            y += row_h;
            row_h = 0;
        }
        if y + h > max_height {
            layer_h = max_height;
            layer += 1;
            x = 0;
            y = 0;
            row_h = 0;
        }
        place.push((layer, x, y));
        x += w;
        row_h = row_h.max(h);
        layer_h = layer_h.max(y + row_h);
    }
    let height = layer_h.max(1);
    let layers = layer + 1;
    let layer_px = (atlas_width * height * 4) as usize;
    let mut rgba = vec![0u8; layer_px * layers as usize];
    let mut sprites = HashMap::new();
    for ((name, pi, e), (ly, ax, ay)) in sel.into_iter().zip(place) {
        let (page, pw) = &pages[pi];
        for yy in 0..e.h as u32 {
            let s = (((e.y as u32 + yy) * pw + e.x as u32) * 4) as usize;
            let d = layer_px * ly as usize + (((ay + yy) * atlas_width + ax) * 4) as usize;
            rgba[d..d + e.w as usize * 4].copy_from_slice(&page[s..s + e.w as usize * 4]);
        }
        sprites.insert(
            name,
            Sprite {
                layer: ly,
                ax,
                ay,
                w: e.w,
                h: e.h,
                off_x: e.off_x,
                off_y: e.off_y,
                orig_w: e.orig_w,
                orig_h: e.orig_h,
            },
        );
    }
    Ok(Atlas {
        rgba,
        width: atlas_width,
        height,
        layers,
        sprites,
    })
}
