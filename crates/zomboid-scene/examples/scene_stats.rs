//! Prints the numbers `tools/web_smoke.mjs` compares the browser build with: scene_stats <game root> <cell x> <cell y>
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (cx, cy): (i32, i32) = (a[2].parse().expect("cell x"), a[3].parse().expect("cell y"));
    let scene = zomboid_scene::scene::load_region(Path::new(&a[1]), cx, cy, cx, cy).expect("scene");
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytemuck::cast_slice::<_, u8>(&scene.instances) {
        h = (h ^ *b as u64).wrapping_mul(0x100000001b3);
    }
    println!(
        "{{\"sprites\":{},\"unresolved\":{},\"atlas\":[{},{},{}],\"levels\":{},\"instanceHash\":\"{:x}\"}}",
        scene.instances.len(),
        scene.unresolved,
        scene.atlas.width,
        scene.atlas.height,
        scene.atlas.layers,
        scene.level_ranges.len() - 1,
        h
    );
}
