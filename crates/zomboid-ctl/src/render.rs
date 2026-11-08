//! CPU reference renderer: draws a window of tiles from a real cell to a PNG (isometric, 2x tile art).
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;
use zomboid_content::{lot::*, pack::*};

struct Atlas {
    pages: Vec<(Vec<u8>, u32, u32)>,
    map: HashMap<String, (usize, SubTexture)>,
}

fn decode(png_bytes: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(png_bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut rd = dec.read_info()?;
    let mut buf = vec![0; rd.output_buffer_size()];
    let info = rd.next_frame(&mut buf)?;
    let (w, h) = (info.width, info.height);
    let px = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buf[..info.buffer_size()]
            .chunks(3)
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf[..info.buffer_size()]
            .chunks(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => buf[..info.buffer_size()]
            .iter()
            .flat_map(|&g| [g, g, g, 255])
            .collect(),
        c => return Err(anyhow!("unsupported png color {c:?}")),
    };
    Ok((px, w, h))
}

fn load_atlas(root: &Path, wanted: &std::collections::HashSet<String>) -> Result<Atlas> {
    let mut files: Vec<_> = std::fs::read_dir(root.join("media/texturepacks"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("pack"))
        .collect();
    // prefer 2x packs: they come first, first definition wins
    files.sort_by_key(|p| {
        let n = p.file_name().unwrap().to_string_lossy().to_string();
        (!n.contains("2x"), n)
    });
    let mut a = Atlas {
        pages: vec![],
        map: HashMap::new(),
    };
    for p in files {
        let b = std::fs::read(&p)?;
        let pages = parse_pack(&b).map_err(|e| anyhow!("{}: {e:?}", p.display()))?;
        for pg in pages {
            let hits: Vec<_> = pg
                .entries
                .iter()
                .filter(|e| wanted.contains(&e.name) && !a.map.contains_key(&e.name))
                .collect();
            if hits.is_empty() {
                continue;
            }
            let d = decode(&b[pg.png.clone()])?;
            a.pages.push(d);
            let idx = a.pages.len() - 1;
            for e in hits {
                a.map.insert(e.name.clone(), (idx, e.clone()));
            }
        }
    }
    Ok(a)
}

/// Render `size`x`size` squares starting at cell-relative (x0,y0) on level 0.
pub fn render(root: &Path, cell: &str, x0: i32, y0: i32, size: i32, out: &Path) -> Result<()> {
    let dir = root.join("media/maps/Muldraugh, KY");
    let hb = std::fs::read(dir.join(format!("{cell}.lotheader")))?;
    let h = parse_lotheader(&hb).map_err(|e| anyhow!("{e}"))?;
    let d = parse_lotheader_dims(&hb, &h).map_err(|e| anyhow!("{e}"))?;
    let pb = std::fs::read(dir.join(format!("world_{cell}.lotpack")))?;
    let pk = parse_lotpack(&pb).map_err(|e| anyhow!("{e}"))?;
    let wanted: std::collections::HashSet<String> = h.tile_names.iter().cloned().collect();
    let atlas = load_atlas(root, &wanted)?;
    let (tw, th) = (128i32, 64i32);
    let (w, hgt) = ((size * tw) + 256, (size * th) + 512);
    let mut img = vec![0u8; (w * hgt * 4) as usize];
    for p in img.chunks_mut(4) {
        p.copy_from_slice(&[20, 24, 20, 255]);
    }
    let mut drawn = 0u64;
    let mut missing = 0u64;
    let mut chunk_cache: HashMap<(i32, i32), Chunk> = HashMap::new();
    for z in d.min_level..=d.max_level {
        for sy in y0..y0 + size {
            for sx in x0..x0 + size {
                let (cx, cy) = (sx / 8, sy / 8);
                let idx = (cx * 32 + cy) as usize;
                if let std::collections::hash_map::Entry::Vacant(v) = chunk_cache.entry((cx, cy)) {
                    v.insert(
                        parse_chunk(&pb, pk.offsets[idx] as usize, d.min_level, d.max_level)
                            .map_err(|e| anyhow!("{e}"))?,
                    );
                }
                let c = &chunk_cache[&(cx, cy)];
                let sq = &c.levels[(z - d.min_level) as usize][((sx % 8) * 8 + (sy % 8)) as usize];
                let Some(sq) = sq else { continue };
                let (lx, ly) = (sx - x0, sy - y0);
                let px = (lx - ly) * (tw / 2) + size * (tw / 2) + 128;
                let py = (lx + ly) * (th / 2) + 384 - z * 192;
                for &t in &sq.tiles {
                    let name = &h.tile_names[t as usize];
                    let Some((pi, e)) = atlas.map.get(name) else {
                        missing += 1;
                        continue;
                    };
                    let (page, pw, _) = &atlas.pages[*pi];
                    let dx0 = px - e.orig_w / 2 + e.off_x;
                    let dy0 = py + th - e.orig_h + e.off_y;
                    for yy in 0..e.h {
                        for xx in 0..e.w {
                            let s = (((e.y + yy) as u32 * pw + (e.x + xx) as u32) * 4) as usize;
                            let a = page[s + 3] as u32;
                            if a == 0 {
                                continue;
                            }
                            let (ox, oy) = (dx0 + xx, dy0 + yy);
                            if ox < 0 || oy < 0 || ox >= w || oy >= hgt {
                                continue;
                            }
                            let o = ((oy * w + ox) * 4) as usize;
                            for k in 0..3 {
                                img[o + k] = ((page[s + k] as u32 * a
                                    + img[o + k] as u32 * (255 - a))
                                    / 255) as u8;
                            }
                        }
                    }
                    drawn += 1;
                }
            }
        }
    }
    let f = std::fs::File::create(out)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w as u32, hgt as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&img)?;
    println!(
        "rendered {drawn} sprites, {missing} unresolved -> {}",
        out.display()
    );
    Ok(())
}
