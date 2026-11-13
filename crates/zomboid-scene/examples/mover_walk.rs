//! Rust side of tests/original_probe/MoverProbe.java: mover_walk <game root> <cell x> <cell y> then per path: x y dx dy steps
use std::path::Path;
use zomboid_scene::scene::{load_region, Mover};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (cx, cy): (i32, i32) = (a[2].parse().expect("cell x"), a[3].parse().expect("cell y"));
    let scene = load_region(Path::new(&a[1]), cx, cy, cx, cy).expect("scene");
    let f = |s: &String| s.parse::<f32>().expect("number");
    for p in a[4..].chunks_exact(5) {
        let (dx, dy, n) = (f(&p[2]), f(&p[3]), p[4].parse::<usize>().expect("steps"));
        let mut m = Mover {
            x: f(&p[0]),
            y: f(&p[1]),
            z: 0.0,
        };
        for leg in 0..2 {
            for _ in 0..n {
                let last = scene.current_square(m.x, m.y, m.z);
                m.x += if leg == 0 { dx } else { -dx };
                m.y += if leg == 0 { dy } else { -dy };
                // The original sets the current square before doStairs changes the height.
                let c = scene.current_square(m.x, m.y, m.z);
                if let Some(last) = last {
                    scene.do_stairs(last, &mut m);
                }
                let c = c.map_or("null".to_string(), |c| format!("{},{},{}", c.0, c.1, c.2));
                println!(
                    "M {:x} {:x} {:x} {c}",
                    m.x.to_bits(),
                    m.y.to_bits(),
                    m.z.to_bits()
                );
            }
        }
    }
}
