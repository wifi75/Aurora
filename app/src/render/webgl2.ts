import type { WaterfallBackend } from './backend';
import { LUT_SIZE, WATERFALL_STOPS, buildLut } from './colormap';
import { RowRing } from './ring';
import { DEFAULT_LEVELS, type Levels } from './levels';
import { RowClock } from './smooth';

export const WATERFALL_WIDTH = 1024;
export const WATERFALL_HEIGHT = 1024;

/** Righe per pixel: una riga per pixel, meno solo se la finestra e' piu' alta dell'anello. */
export function rowsPerPixel(heightPx: number): number {
  const usable = WATERFALL_HEIGHT - 2;
  return heightPx > usable ? usable / heightPx : 1;
}

const VERTEX = `#version 300 es
out vec2 v_uv;
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  v_uv = vec2(p.x, 1.0 - p.y);
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}`;

// La riga piu' recente sta in alto e le altre scendono: lo scorrimento e' solo un offset sull'anello, nessuna ricopiatura.
// Una riga vale un pixel (niente compressione verticale che farebbe tremolare i segnali); u_frac e' la frazione di riga
// gia' scorsa dall'ultima riga arrivata, cosi' la cascata scende in modo continuo. In orizzontale si mediano quattro
// punti (e si tiene anche il massimo) perche' i 1024 canali sono piu' dei pixel e un segnale stretto non sparisca ne' tremi.
const FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_uv;
out vec4 o;
uniform sampler2D u_tex;
uniform sampler2D u_lut;
uniform float u_head;
uniform float u_h;
uniform float u_frac;
uniform float u_rpp;
uniform vec2 u_px;
uniform vec3 u_levels;
float at(float x, float r) {
  return texture(u_tex, vec2(x, r)).r;
}
void main() {
  float py = v_uv.y * u_px.y * u_rpp;
  float coord = min(u_head - py + u_frac, u_head - 0.5);
  float r = fract(coord / u_h);
  float w = 1.0 / u_px.x;
  float a = at(v_uv.x - 0.375 * w, r);
  float b = at(v_uv.x - 0.125 * w, r);
  float c = at(v_uv.x + 0.125 * w, r);
  float d = at(v_uv.x + 0.375 * w, r);
  float v = 0.5 * (a + b + c + d) * 0.25 + 0.5 * max(max(a, b), max(c, d));
  float x = clamp((v - u_levels.x) / max(1.0 - u_levels.x, 0.05), 0.0, 1.0);
  x = pow(x, 1.0 / u_levels.y);
  x = clamp((x - 0.5) * u_levels.z + 0.5, 0.0, 1.0);
  o = vec4(texture(u_lut, vec2(x * 0.99609375 + 0.001953125, 0.5)).rgb, 1.0);
}`;

function compile(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) throw new Error('shader non creato');
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(shader) ?? '';
    gl.deleteShader(shader);
    throw new Error(`shader non compilato: ${log}`);
  }
  return shader;
}

export class WebGl2Waterfall implements WaterfallBackend {
  readonly kind = 'webgl2' as const;
  private canvas!: HTMLCanvasElement;
  private gl!: WebGL2RenderingContext;
  private program: WebGLProgram | null = null;
  private tex: WebGLTexture | null = null;
  private lut: WebGLTexture | null = null;
  private uHead: WebGLUniformLocation | null = null;
  private uH: WebGLUniformLocation | null = null;
  private uFrac: WebGLUniformLocation | null = null;
  private uRpp: WebGLUniformLocation | null = null;
  private uPx: WebGLUniformLocation | null = null;
  private uLevels: WebGLUniformLocation | null = null;
  private levels: Levels = DEFAULT_LEVELS;
  private readonly clock = new RowClock();
  private raf = 0;
  private readonly ring = new RowRing(WATERFALL_WIDTH, WATERFALL_HEIGHT);

  async init(canvas: HTMLCanvasElement): Promise<void> {
    const gl = canvas.getContext('webgl2', { antialias: false, alpha: false, powerPreference: 'high-performance' });
    if (!gl) throw new Error('WebGL2 non disponibile');
    this.canvas = canvas;
    this.gl = gl;

    const program = gl.createProgram();
    if (!program) throw new Error('programma non creato');
    const vs = compile(gl, gl.VERTEX_SHADER, VERTEX);
    const fs = compile(gl, gl.FRAGMENT_SHADER, FRAGMENT);
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);
    gl.deleteShader(vs);
    gl.deleteShader(fs);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      throw new Error(`programma non collegato: ${gl.getProgramInfoLog(program) ?? ''}`);
    }
    this.program = program;
    this.uHead = gl.getUniformLocation(program, 'u_head');
    this.uH = gl.getUniformLocation(program, 'u_h');
    this.uFrac = gl.getUniformLocation(program, 'u_frac');
    this.uRpp = gl.getUniformLocation(program, 'u_rpp');
    this.uPx = gl.getUniformLocation(program, 'u_px');
    this.uLevels = gl.getUniformLocation(program, 'u_levels');

    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    this.tex = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.tex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, WATERFALL_WIDTH, WATERFALL_HEIGHT, 0, gl.RED, gl.UNSIGNED_BYTE, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);

    this.lut = gl.createTexture();
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.lut);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, LUT_SIZE, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, buildLut(WATERFALL_STOPS));
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    gl.useProgram(program);
    gl.uniform1i(gl.getUniformLocation(program, 'u_tex'), 0);
    gl.uniform1i(gl.getUniformLocation(program, 'u_lut'), 1);
    this.schedule();
  }

  pushRow(row: Uint8Array): void {
    const index = this.ring.push(row);
    if (index < 0) return;
    this.clock.push(performance.now());
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.tex);
    gl.texSubImage2D(
      gl.TEXTURE_2D, 0, 0, index, WATERFALL_WIDTH, 1, gl.RED, gl.UNSIGNED_BYTE,
      this.ring.data.subarray(index * WATERFALL_WIDTH, (index + 1) * WATERFALL_WIDTH),
    );
    this.schedule();
  }

  resize(widthPx: number, heightPx: number): void {
    this.canvas.width = Math.max(1, Math.floor(widthPx));
    this.canvas.height = Math.max(1, Math.floor(heightPx));
    this.schedule();
  }

  destroy(): void {
    if (this.raf) cancelAnimationFrame(this.raf);
    this.raf = 0;
    const gl = this.gl;
    if (!gl) return;
    gl.deleteTexture(this.tex);
    gl.deleteTexture(this.lut);
    gl.deleteProgram(this.program);
    this.tex = this.lut = this.program = null;
  }

  private schedule(): void {
    if (this.raf || !this.program) return;
    this.raf = requestAnimationFrame(() => {
      this.raf = 0;
      this.draw();
    });
  }

  setLevels(levels: Levels): void {
    this.levels = levels;
  }

  private draw(): void {
    const gl = this.gl;
    if (!this.program) return;
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.useProgram(this.program);
    gl.uniform1f(this.uHead, this.ring.head);
    gl.uniform1f(this.uH, WATERFALL_HEIGHT);
    gl.uniform1f(this.uFrac, this.clock.fraction(performance.now()));
    gl.uniform1f(this.uRpp, rowsPerPixel(this.canvas.height));
    gl.uniform2f(this.uPx, this.canvas.width, this.canvas.height);
    gl.uniform3f(this.uLevels, this.levels.black, this.levels.gain, this.levels.contrast);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    // la cascata scende di continuo: si ridisegna a ogni fotogramma
    this.schedule();
  }
}
