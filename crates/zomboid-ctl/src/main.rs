mod render;
use anyhow::{bail, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_default();
    let root: PathBuf = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".."));
    match cmd.as_str() {
        "save-time" => {
            let file = if root.is_dir() {
                root.join("map_t.bin")
            } else {
                root
            };
            let time = zomboid_content::save::load_game_time(&file).map_err(anyhow::Error::msg)?;
            println!("save time {:02}:{:02} (raw {:.8}), year {}, month index {}, day index {}, version {}",
                time.hour(), time.minutes(), time.time_of_day, time.year, time.month_index, time.day_index, time.version);
            Ok(())
        }
        "check" => {
            let inv = zomboid_content::inventory(&root);
            println!("== media/ files by top directory ==");
            for (k, v) in &inv.by_dir {
                println!("{k:24} {v}");
            }
            println!("== by extension ==");
            for (k, v) in &inv.by_ext {
                println!("{k:12} {v}");
            }
            println!("== maps ==");
            for (k, m) in &inv.maps {
                println!(
                    "{k}: headers={} packs={} header_no_pack={} pack_no_header={}",
                    m.lotheaders,
                    m.lotpacks,
                    m.headers_without_pack.len(),
                    m.packs_without_header.len()
                );
            }
            if !inv.unreadable.is_empty() {
                println!("unreadable: {:?}", inv.unreadable);
            }
            Ok(())
        }
        "lots" => {
            use zomboid_content::lot::*;
            let dir = root.join("media/maps/Muldraugh, KY");
            let (mut ok, mut bad) = (0u32, 0u32);
            for e in std::fs::read_dir(&dir)?.flatten() {
                let p = e.path();
                if !p.is_file() {
                    continue;
                }
                let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
                let b = std::fs::read(&p)?;
                let r = match ext {
                    "lotpack" => parse_lotpack(&b).map(|_| ()),
                    "lotheader" => parse_lotheader(&b).map(|_| ()),
                    _ => continue,
                };
                match r {
                    Ok(()) => ok += 1,
                    Err(e) => {
                        bad += 1;
                        if bad <= 5 {
                            println!("{}: {e}", p.display());
                        }
                    }
                }
            }
            println!("lot files parsed ok={ok} failed={bad}");
            let (mut chunks, mut mism, mut tiles) = (0u64, 0u64, 0u64);
            for e in std::fs::read_dir(&dir)?.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("lotpack") {
                    continue;
                }
                let name = p
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .replace("world_", "")
                    .replace(".lotpack", ".lotheader");
                let hb = std::fs::read(dir.join(&name))?;
                let h = parse_lotheader(&hb)?;
                let d = parse_lotheader_dims(&hb, &h)?;
                let pb = std::fs::read(&p)?;
                let pk = parse_lotpack(&pb)?;
                let mut ends: Vec<u64> = pk.offsets.clone();
                ends.push(pb.len() as u64);
                ends.sort();
                ends.dedup();
                for &off in &pk.offsets {
                    let c = parse_chunk(&pb, off as usize, d.min_level, d.max_level)?;
                    let end = ends[ends.partition_point(|&x| x <= off)];
                    chunks += 1;
                    tiles += c
                        .levels
                        .iter()
                        .flatten()
                        .flatten()
                        .map(|s| s.tiles.len() as u64)
                        .sum::<u64>();
                    if off + c.bytes_read as u64 != end {
                        mism += 1;
                        if mism <= 5 {
                            println!(
                                "{}: chunk read {} expected {}",
                                name,
                                c.bytes_read,
                                end - off
                            );
                        }
                    }
                }
            }
            println!("chunks={chunks} length_mismatch={mism} tiles={tiles}");
            Ok(())
        }
        "hist" => {
            use zomboid_content::lot::*;
            let dir = root.join("media/maps/Muldraugh, KY");
            for cell in ["0_18", "0_20", "30_20"] {
                let hb = std::fs::read(dir.join(format!("{cell}.lotheader")))?;
                let h = parse_lotheader(&hb)?;
                let d = parse_lotheader_dims(&hb, &h)?;
                let pb = std::fs::read(dir.join(format!("world_{cell}.lotpack")))?;
                let pk = parse_lotpack(&pb)?;
                let mut mx = -1;
                let mut mn = i32::MAX;
                let mut rooms = std::collections::BTreeSet::new();
                for &o in &pk.offsets {
                    let c = parse_chunk(&pb, o as usize, d.min_level, d.max_level)?;
                    for s in c.levels.iter().flatten().flatten() {
                        rooms.insert(s.room_id);
                        for &t in &s.tiles {
                            mx = mx.max(t);
                            mn = mn.min(t);
                        }
                    }
                }
                println!("{cell}: names={} dims={:?} tile range {mn}..={mx} distinct room ids={} first names={:?}", h.tile_names.len(), d, rooms.len(), &h.tile_names[..h.tile_names.len().min(3)]);
            }
            Ok(())
        }
        "packs" => {
            let dir = root.join("media/texturepacks");
            let (mut ok, mut bad, mut sub) = (0, 0, 0usize);
            for e in std::fs::read_dir(&dir)?.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("pack") {
                    continue;
                }
                let b = std::fs::read(&p)?;
                match zomboid_content::pack::parse_pack(&b) {
                    Ok(pg) => {
                        ok += 1;
                        sub += pg.iter().map(|g| g.entries.len()).sum::<usize>();
                    }
                    Err(e) => {
                        bad += 1;
                        println!("{}: {e:?}", p.display());
                    }
                }
            }
            println!("packs ok={ok} failed={bad} subtextures={sub}");
            Ok(())
        }
        "resolve" => {
            use std::collections::BTreeSet;
            let mut known = BTreeSet::new();
            for e in std::fs::read_dir(root.join("media/texturepacks"))?.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("pack") {
                    continue;
                }
                for pg in zomboid_content::pack::parse_pack(&std::fs::read(&p)?)
                    .map_err(|e| anyhow::anyhow!("{e:?}"))?
                {
                    for s in pg.entries {
                        known.insert(s.name);
                    }
                }
            }
            let mut used = BTreeSet::new();
            let dir = root.join("media/maps/Muldraugh, KY");
            for e in std::fs::read_dir(&dir)?.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("lotheader") {
                    continue;
                }
                for n in zomboid_content::lot::parse_lotheader(&std::fs::read(&p)?)?.tile_names {
                    used.insert(n);
                }
            }
            let missing: Vec<_> = used.iter().filter(|n| !known.contains(*n)).collect();
            println!(
                "distinct tile names used={} known pack textures={} unresolved={}",
                used.len(),
                known.len(),
                missing.len()
            );
            for m in &missing {
                println!("  missing: {m}");
            }
            Ok(())
        }
        "rooms" => {
            // rooms <root> <cell> [lx ly]: named rooms from a cell header; with lx ly, the rooms containing that cell-local square
            let a: Vec<String> = std::env::args().collect();
            let cell = a.get(3).cloned().unwrap_or("32_45".into());
            let b = std::fs::read(
                std::path::Path::new(&a[2])
                    .join("media/maps/Muldraugh, KY")
                    .join(format!("{cell}.lotheader")),
            )?;
            let h = zomboid_content::lot::parse_lotheader(&b)?;
            let (rooms, end) = zomboid_content::lot::parse_rooms(&b, &h)?;
            println!(
                "{cell}: {} rooms, parsed to byte {end} of {}; rooms with objects {}; levels {:?}",
                rooms.len(),
                b.len(),
                rooms.iter().filter(|r| !r.objects.is_empty()).count(),
                rooms
                    .iter()
                    .map(|r| r.level)
                    .collect::<std::collections::BTreeSet<_>>()
            );
            if let (Some(lx), Some(ly)) = (
                a.get(4).and_then(|s| s.parse::<i32>().ok()),
                a.get(5).and_then(|s| s.parse::<i32>().ok()),
            ) {
                for r in rooms.iter().filter(|r| r.contains(lx, ly)) {
                    println!("  ({lx},{ly}) in room `{}` rects {:?}", r.name, r.rects);
                }
            }
            Ok(())
        }
        "zones" => {
            // zones <root> [map dir name]: type histogram of media/maps/<map>/objects.lua
            let a: Vec<String> = std::env::args().collect();
            let map = a.get(3).cloned().unwrap_or("Muldraugh, KY".into());
            let z = zomboid_content::zones::parse_objects(&std::fs::read(
                std::path::Path::new(&a[2])
                    .join("media/maps")
                    .join(&map)
                    .join("objects.lua"),
            )?)
            .map_err(|e| anyhow::anyhow!(e))?;
            let mut h = std::collections::BTreeMap::<String, usize>::new();
            for q in &z {
                *h.entry(q.kind.clone()).or_default() += 1;
            }
            println!(
                "{map}: {} zones, {} polygons",
                z.len(),
                z.iter().filter(|q| !q.points.is_empty()).count()
            );
            for (k, n) in h {
                println!("{n:7} {k}");
            }
            Ok(())
        }
        "thumb" => {
            // thumb <root> <in.png> <out.png> <width>: area-average downscale (for the website gallery)
            let a: Vec<String> = std::env::args().collect();
            let (rgba, w, h) = zomboid_content::atlas::decode_png(&std::fs::read(&a[3])?)
                .map_err(|e| anyhow::anyhow!(e))?;
            let ch = rgba.len() / (w as usize * h as usize);
            let tw: usize = a[5].parse()?;
            let th = (h as usize * tw) / w as usize;
            let mut out = vec![0u8; tw * th * 3];
            for y in 0..th {
                let (y0, y1) = (
                    y * h as usize / th,
                    ((y + 1) * h as usize / th).max(y * h as usize / th + 1),
                );
                for x in 0..tw {
                    let (x0, x1) = (
                        x * w as usize / tw,
                        ((x + 1) * w as usize / tw).max(x * w as usize / tw + 1),
                    );
                    let mut s = [0u32; 3];
                    let mut n = 0u32;
                    for yy in y0..y1 {
                        for xx in x0..x1 {
                            let k = (yy * w as usize + xx) * ch;
                            for c in 0..3 {
                                s[c] += rgba[k + c.min(ch - 1)] as u32;
                            }
                            n += 1;
                        }
                    }
                    for c in 0..3 {
                        out[(y * tw + x) * 3 + c] = (s[c] / n.max(1)) as u8;
                    }
                }
            }
            let f = std::fs::File::create(&a[4])?;
            let mut enc = png::Encoder::new(std::io::BufWriter::new(f), tw as u32, th as u32);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            enc.set_compression(png::Compression::Best);
            enc.write_header()?.write_image_data(&out)?;
            println!("{} -> {} ({tw}x{th})", a[3], a[4]);
            Ok(())
        }
        "imgdiff" => {
            // imgdiff <game.png> <port.png>: mean abs luma diff on a 4x box-downsampled image, with a best-shift search.
            let a: Vec<String> = std::env::args().collect();
            let load = |p: &str| -> anyhow::Result<(Vec<f32>, usize, usize)> {
                let (rgba, w, h) = zomboid_content::atlas::decode_png(&std::fs::read(p)?)
                    .map_err(|e| anyhow::anyhow!(e))?;
                let ch = rgba.len() / (w as usize * h as usize);
                let (dw, dh) = (w as usize / 4, h as usize / 4);
                let mut o = vec![0f32; dw * dh];
                for y in 0..dh {
                    for x in 0..dw {
                        let mut s = 0f32;
                        for j in 0..4 {
                            for i in 0..4 {
                                let k = ((y * 4 + j) * w as usize + x * 4 + i) * ch;
                                s += 0.299 * rgba[k] as f32
                                    + 0.587 * rgba[k + 1.min(ch - 1)] as f32
                                    + 0.114 * rgba[k + 2.min(ch - 1)] as f32;
                            }
                        }
                        o[y * dw + x] = s / 16.0;
                    }
                }
                Ok((o, dw, dh))
            };
            let (g, w, h) = load(a.get(3).ok_or_else(|| anyhow::anyhow!("need game.png"))?)?;
            let (p, w2, h2) = load(a.get(4).ok_or_else(|| anyhow::anyhow!("need port.png"))?)?;
            anyhow::ensure!(w == w2 && h == h2, "size mismatch {w}x{h} vs {w2}x{h2}");
            let reg: Vec<i32> = (5..9)
                .filter_map(|i| a.get(i).and_then(|s| s.parse::<i32>().ok()))
                .map(|v| v / 4)
                .collect();
            let (rx0, ry0, rx1, ry1) = if reg.len() == 4 {
                (reg[0], reg[1], reg[2], reg[3])
            } else {
                (0, 0, w as i32, h as i32)
            };
            let diff = |dx: i32, dy: i32| -> f32 {
                let (mut s, mut n) = (0f64, 0u32);
                for y in ry0..ry1 {
                    let y2 = y + dy;
                    if y2 < 0 || y2 >= h as i32 {
                        continue;
                    }
                    for x in rx0..rx1 {
                        let x2 = x + dx;
                        if x2 < 0 || x2 >= w as i32 {
                            continue;
                        }
                        s += (g[y as usize * w + x as usize] - p[y2 as usize * w + x2 as usize])
                            .abs() as f64;
                        n += 1;
                    }
                }
                (s / n.max(1) as f64) as f32
            };
            let base = diff(0, 0);
            let mut best = (base, 0, 0);
            for dy in -80..=80 {
                for dx in -60..=60 {
                    let d = diff(dx, dy);
                    if d < best.0 {
                        best = (d, dx, dy);
                    }
                }
            }
            println!("downsampled {w}x{h}: mean|luma diff| at shift 0,0 = {base:.2}; best = {:.2} at port shift dx={} dy={} (x4 px)", best.0, best.1 * 4, best.2 * 4);
            Ok(())
        }
        "render" => {
            let a: Vec<String> = std::env::args().collect();
            let cell = a.get(3).cloned().unwrap_or("41_39".into());
            let n = |i: usize, d: i32| a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
            render::render(
                &root,
                &cell,
                n(4, 0),
                n(5, 0),
                n(6, 32),
                std::path::Path::new(&a.get(7).cloned().unwrap_or("out.png".into())),
            )
        }
        "tiledefs" => {
            let d = zomboid_content::tiledefs::TileDefs::load(&root)?;
            println!("tile definitions: {}", d.tiles.len());
            for p in [
                "solid",
                "solidtrans",
                "WallN",
                "WallW",
                "collideN",
                "collideW",
                "water",
                "IsWater",
                "container",
            ] {
                println!(
                    "  {p:12} {}",
                    d.tiles.values().filter(|t| t.contains_key(p)).count()
                );
            }
            for (label, required) in [
                ("solidtrans+window", &["solidtrans", "windowN"][..]),
                ("solidtrans+Window", &["solidtrans", "WindowN"][..]),
                ("water+solidtrans", &["water", "solidtrans"][..]),
                ("windowN+collideN", &["windowN", "collideN"][..]),
                ("windowW+collideW", &["windowW", "collideW"][..]),
            ] {
                let mut names: Vec<_> = d
                    .tiles
                    .iter()
                    .filter(|(_, props)| required.iter().all(|p| props.contains_key(*p)))
                    .map(|(name, _)| name.as_str())
                    .collect();
                names.sort_unstable();
                println!(
                    "  {label:22} {} examples: {}",
                    names.len(),
                    names.iter().take(3).copied().collect::<Vec<_>>().join(", ")
                );
            }
            Ok(())
        }
        "scripts" => {
            fn walk(d: &std::path::Path, o: &mut Vec<std::path::PathBuf>) {
                if let Ok(rd) = std::fs::read_dir(d) {
                    for e in rd.flatten() {
                        let p = e.path();
                        if p.is_dir() {
                            walk(&p, o)
                        } else if p.extension().and_then(|x| x.to_str()) == Some("txt") {
                            o.push(p)
                        }
                    }
                }
            }
            let mut files = vec![];
            walk(&root.join("media/scripts"), &mut files);
            let (mut ok, mut bad) = (0, 0);
            let mut kinds = std::collections::BTreeMap::<String, u64>::new();
            fn count(
                b: &zomboid_content::script::Block,
                k: &mut std::collections::BTreeMap<String, u64>,
            ) {
                *k.entry(b.kind.clone()).or_default() += 1;
                for c in &b.children {
                    count(c, k);
                }
            }
            for f in &files {
                match zomboid_content::script::parse_script(
                    &std::fs::read_to_string(f).unwrap_or_default(),
                ) {
                    Ok(t) => {
                        ok += 1;
                        for b in &t {
                            count(b, &mut kinds);
                        }
                    }
                    Err(e) => {
                        bad += 1;
                        if bad <= 5 {
                            println!("{}: line {} {}", f.display(), e.line, e.msg);
                        }
                    }
                }
            }
            println!("script files={} parsed ok={ok} failed={bad}", files.len());
            let mut v: Vec<_> = kinds.into_iter().collect();
            v.sort_by_key(|x| std::cmp::Reverse(x.1));
            println!(
                "{}",
                v.iter()
                    .take(25)
                    .map(|(k, n)| format!("{k}:{n}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            Ok(())
        }
        _ => bail!("usage: zomboid-ctl check <game-root>"),
    }
}
