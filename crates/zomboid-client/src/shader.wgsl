struct Cam { center: vec2<f32>, scale: vec2<f32> };
@group(0) @binding(0) var<uniform> cam: Cam;
@group(0) @binding(1) var atlas: texture_2d_array<f32>;
@group(0) @binding(2) var samp: sampler;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: i32,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, @location(0) rect: vec4<f32>, @location(1) uv: vec4<f32>, @location(2) layer: f32) -> VOut {
    let corner = vec2<f32>(f32(vi & 1u), f32((vi >> 1u) & 1u));
    let world = rect.xy + corner * rect.zw;
    let ndc = (world - cam.center) * cam.scale * vec2<f32>(1.0, -1.0);
    var o: VOut;
    o.pos = vec4<f32>(ndc, 0.0, 1.0);
    o.uv = mix(uv.xy, uv.zw, corner);
    o.layer = i32(layer + 0.5);
    return o;
}

@fragment
fn fs_main(i: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(atlas, samp, i.uv, i.layer);
    if (c.a < 0.004) { discard; }
    return c;
}
