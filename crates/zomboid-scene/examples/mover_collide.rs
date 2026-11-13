//! Rust side of tests/original_probe/MoverCollideProbe.java: mover_collide <game root> <cell x> <cell y> then per path: x y vx vy ticks
use std::path::Path;
use zomboid_scene::scene::{load_region, MovingObject};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (cx, cy): (i32, i32) = (a[2].parse().expect("cell x"), a[3].parse().expect("cell y"));
    let scene = load_region(Path::new(&a[1]), cx, cy, cx, cy).expect("scene");
    let f = |s: &String| s.parse::<f32>().expect("number");
    for p in a[4..].chunks_exact(5) {
        let (x, y, vx, vy, n) = (
            f(&p[0]),
            f(&p[1]),
            f(&p[2]),
            f(&p[3]),
            p[4].parse::<usize>().expect("ticks"),
        );
        let mut o = MovingObject::new(x, y, 0.0, scene.current_square(x, y, 0.0));
        for _ in 0..n {
            o.lx = o.x;
            o.ly = o.y;
            o.nx = o.x + vx;
            o.ny = o.y + vy;
            scene.postupdate(&mut o);
            let c = o
                .current
                .map_or("null".to_string(), |c| format!("{},{},{}", c.0, c.1, c.2));
            let fl = |b: bool, ch: char| if b { ch } else { '-' };
            println!(
                "M {:x} {:x} {:x} {c} {}{}{}{}",
                o.x.to_bits(),
                o.y.to_bits(),
                o.z.to_bits(),
                fl(o.collided_n, 'N'),
                fl(o.collided_s, 'S'),
                fl(o.collided_w, 'W'),
                fl(o.collided_e, 'E')
            );
        }
    }
}
