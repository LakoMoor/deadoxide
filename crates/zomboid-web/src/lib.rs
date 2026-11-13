//! Browser entry points. The page mounts files from the player's own game folder into the engine's in-memory file
//! system, then asks for a scene: the same `load_region` the desktop client uses (ported engine loader, collision
//! matrices, door/window blocking). Drawing is done by the page with WebGL from the instance and atlas data.
use std::path::Path;

use wasm_bindgen::prelude::*;
use zomboid_engine::vfs;
use zomboid_scene::scene::{iso, load_region, Mover, Scene};

/// Root under which the page mounts files (`game/media/...`).
const ROOT: &str = "game";

/// Mount one file of the player's install, `path` relative to the game root (e.g. `media/newtiledefinitions.tiles`).
#[wasm_bindgen]
pub fn mount(path: &str, data: Vec<u8>) {
    vfs::mount(&format!("{ROOT}/{path}"), data);
}

#[wasm_bindgen]
pub struct WebScene {
    scene: Scene,
    player: Mover,
}

#[wasm_bindgen]
impl WebScene {
    /// Build the scene of one 256x256 cell and put the player at (x, y, z).
    #[wasm_bindgen(constructor)]
    pub fn new(cell_x: i32, cell_y: i32, x: f32, y: f32, z: i32) -> Result<WebScene, JsError> {
        let scene = load_region(Path::new(ROOT), cell_x, cell_y, cell_x, cell_y)
            .map_err(|e| JsError::new(&format!("{e:#}")))?;
        Ok(WebScene {
            scene,
            player: Player { x, y, z },
        })
    }
    /// 9 floats per sprite: rect x, y, w, h (world pixels), uv u0, v0, u1, v1, atlas layer. Painter's order.
    pub fn instances(&self) -> Vec<f32> {
        bytemuck::cast_slice(&self.scene.instances).to_vec()
    }
    /// Index of the first instance of each level, plus the total.
    pub fn level_ranges(&self) -> Vec<u32> {
        self.scene.level_ranges.clone()
    }
    pub fn min_level(&self) -> i32 {
        self.scene.min_level
    }
    pub fn unresolved(&self) -> f64 {
        self.scene.unresolved as f64
    }
    pub fn atlas_width(&self) -> u32 {
        self.scene.atlas.width
    }
    pub fn atlas_height(&self) -> u32 {
        self.scene.atlas.height
    }
    pub fn atlas_layers(&self) -> u32 {
        self.scene.atlas.layers
    }
    /// RGBA pixels of one atlas layer.
    pub fn atlas_layer(&self, layer: u32) -> Vec<u8> {
        let n = (self.scene.atlas.width * self.scene.atlas.height * 4) as usize;
        let start = layer as usize * n;
        self.scene
            .atlas
            .rgba
            .get(start..start + n)
            .map(<[u8]>::to_vec)
            .unwrap_or_default()
    }
    /// Free the atlas pixels once the page has uploaded them to the GPU.
    pub fn drop_atlas_pixels(&mut self) {
        self.scene.atlas.rgba = Vec::new();
    }
    pub fn player_x(&self) -> f32 {
        self.player.x
    }
    pub fn player_y(&self) -> f32 {
        self.player.y
    }
    /// The floor the player is on.
    pub fn player_z(&self) -> i32 {
        self.player.z.floor() as i32
    }
    /// Continuous height (fractional on stairs).
    pub fn player_height(&self) -> f32 {
        self.player.z
    }
    /// World-pixel position of the player's feet (camera target).
    pub fn player_px(&self) -> Vec<f32> {
        iso(self.player.x, self.player.y, self.player_z()).to_vec()
    }
    /// The 9 floats of the player marker sprite.
    pub fn player_marker(&self) -> Vec<f32> {
        let m = self
            .scene
            .marker("__player", self.player.x, self.player.y, self.player.z);
        bytemuck::bytes_of(&m)
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect()
    }
    /// The interact key: open or close the nearest ordinary door within reach. After `true` the page must fetch
    /// `instances()` and `level_ranges()` again.
    pub fn interact(&mut self) -> bool {
        self.scene
            .toggle_door_near(self.player.x, self.player.y, self.player.z)
    }
    /// Move the player for `dt` seconds in screen direction (sx, sy), each -1, 0 or 1, at 4 squares per second
    /// (a placeholder speed, as in the desktop client). Collision is the ported one and stairs change the floor. Returns whether it moved.
    pub fn step(&mut self, sx: f32, sy: f32, dt: f32) -> bool {
        // Screen axes are the world axes rotated by 45 degrees.
        let (wx, wy) = (sx + sy, sy - sx);
        let len = (wx * wx + wy * wy).sqrt();
        if len == 0.0 {
            return false;
        }
        let dist = 4.0 * dt.min(0.25);
        let n = (dist / 0.2).ceil().max(1.0) as i32;
        let (dx, dy) = (wx / len * dist / n as f32, wy / len * dist / n as f32);
        let mut moved = false;
        for _ in 0..n {
            moved |= self.scene.move_character(&mut self.player, dx, dy);
        }
        moved
    }
}
