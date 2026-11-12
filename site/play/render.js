// WebGL2 renderer for the instance list the wasm scene produces. Mirrors crates/zomboid-client/src/shader.wgsl:
// one textured quad per instance from a texture array, alpha blending, painter's order.

const VS = `#version 300 es
layout(location=0) in vec4 rect;
layout(location=1) in vec4 uv;
layout(location=2) in float layer;
uniform vec2 center, scale;
out vec2 vUv; flat out int vLayer;
void main() {
  vec2 corner = vec2(float(gl_VertexID & 1), float((gl_VertexID >> 1) & 1));
  vec2 world = rect.xy + corner * rect.zw;
  gl_Position = vec4((world - center) * scale * vec2(1.0, -1.0), 0.0, 1.0);
  vUv = mix(uv.xy, uv.zw, corner);
  vLayer = int(layer + 0.5);
}`;
const FS = `#version 300 es
precision mediump float; precision mediump sampler2DArray;
in vec2 vUv; flat in int vLayer;
uniform sampler2DArray atlas; uniform float light;
out vec4 o;
void main() {
  vec4 c = texture(atlas, vec3(vUv, float(vLayer)));
  if (c.a < 0.004) discard;
  o = vec4(c.rgb * light, c.a);
}`;

function shader(gl, type, src) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
  return s;
}

export class Renderer {
  constructor(canvas) {
    const gl = (this.gl = canvas.getContext("webgl2", { antialias: false, alpha: false }));
    if (!gl) throw new Error("WebGL2 is not available in this browser");
    this.canvas = canvas;
    const p = (this.program = gl.createProgram());
    gl.attachShader(p, shader(gl, gl.VERTEX_SHADER, VS));
    gl.attachShader(p, shader(gl, gl.FRAGMENT_SHADER, FS));
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
    this.u = { center: gl.getUniformLocation(p, "center"), scale: gl.getUniformLocation(p, "scale"), light: gl.getUniformLocation(p, "light") };
    this.maxTexture = gl.getParameter(gl.MAX_TEXTURE_SIZE);
    this.count = 0;
  }

  /** layers: function(i) -> Uint8Array RGBA of width*height. */
  setAtlas(width, height, layerCount, layers) {
    const gl = this.gl;
    if (width > this.maxTexture || height > this.maxTexture) throw new Error(`atlas ${width}x${height} exceeds this GPU's texture limit ${this.maxTexture}`);
    this.texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D_ARRAY, this.texture);
    gl.texStorage3D(gl.TEXTURE_2D_ARRAY, 1, gl.RGBA8, width, height, layerCount);
    for (let i = 0; i < layerCount; i++) gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, 0, 0, 0, i, width, height, 1, gl.RGBA, gl.UNSIGNED_BYTE, layers(i));
    for (const [k, v] of [[gl.TEXTURE_MIN_FILTER, gl.LINEAR], [gl.TEXTURE_MAG_FILTER, gl.NEAREST], [gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE], [gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE]]) gl.texParameteri(gl.TEXTURE_2D_ARRAY, k, v);
  }

  /** instances: Float32Array, 9 floats each, with one spare slot at the end for the player marker. */
  setInstances(instances) {
    const gl = this.gl;
    this.count = instances.length / 9 - 1;
    this.vao = gl.createVertexArray();
    gl.bindVertexArray(this.vao);
    this.buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    gl.bufferData(gl.ARRAY_BUFFER, instances, gl.DYNAMIC_DRAW);
    for (const [loc, size, off] of [[0, 4, 0], [1, 4, 16], [2, 1, 32]]) {
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, 36, off);
      gl.vertexAttribDivisor(loc, 1);
    }
  }

  setMarker(marker) {
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    gl.bufferSubData(gl.ARRAY_BUFFER, this.count * 36, marker);
  }

  /** Draw instances [0, end) and then the marker; camera centre in world pixels, zoom in screen px per world px. */
  draw(center, zoom, end, clear = [20 / 255, 24 / 255, 20 / 255]) {
    const gl = this.gl, c = this.canvas;
    gl.viewport(0, 0, c.width, c.height);
    gl.clearColor(clear[0], clear[1], clear[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.enable(gl.BLEND);
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.useProgram(this.program);
    gl.uniform2f(this.u.center, center[0], center[1]);
    gl.uniform2f(this.u.scale, (2 * zoom) / c.width, (2 * zoom) / c.height);
    gl.uniform1f(this.u.light, 1);
    gl.bindVertexArray(this.vao);
    gl.bindTexture(gl.TEXTURE_2D_ARRAY, this.texture);
    gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, Math.min(end, this.count));
    // the player marker is the spare last instance of the buffer
    this.drawRange(this.count, 1);
  }

  drawRange(first, n) {
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    for (const [loc, size, off] of [[0, 4, 0], [1, 4, 16], [2, 1, 32]]) gl.vertexAttribPointer(loc, size, gl.FLOAT, false, 36, off + first * 36);
    gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, n);
    for (const [loc, size, off] of [[0, 4, 0], [1, 4, 16], [2, 1, 32]]) gl.vertexAttribPointer(loc, size, gl.FLOAT, false, 36, off);
  }
}
