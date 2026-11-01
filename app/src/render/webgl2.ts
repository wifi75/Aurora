import type { WaterfallBackend } from './backend';
import { LUT_SIZE, jtdxPaletteLut } from './colormap';
import { sharedRing } from './ring';
import { Y_MIN, Y_PER_DB } from './jtdxRow';
import { JTDX_PALETTES, DEFAULT_PALETTE } from './wfPalettes';
import { RowClock } from './smooth';

/** Colonne della texture: Bins/Pixel 1 su 3000 Hz sono 4096 canali; con piu' colonne la riga si riduce (massimo) prima di entrare. */
export const WATERFALL_WIDTH = 4096;
export const WATERFALL_HEIGHT = 1024;

/**
 * Il colore di `CPlotter::draw` come due numeri per lo shader: `indice = a * y + b` con `a = 10 * 10^(0,02 * Gain)` e
 * `b = 10 * Zero + 40`. Gain e Zero sono i valori dei cursori di JTDX (-50...50).
 */
export function toneUniform(gain: number, zero: number): [number, number] {
  return [10 * 10 ** (0.02 * gain), 10 * zero + 40];
}

/** Il tipo delle righe di periodo, per chi le disegna (`flag` di `pushRow`). */
export const ROW_FLAG = { none: 0, rx: 1, tx: 2 } as const;

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
// gia' scorsa dall'ultima riga arrivata, cosi' la cascata scende in modo continuo. La texture tiene il livello `y` del Wide Graph
// di JTDX (byte = (y + 24) * 4); il colore e' quello di `CPlotter::draw`: `10 * gain * y + 10 * zero + 40` tenuto in 0-254,
// cercato nella tavolozza da 256 colori. In orizzontale si mediano quattro punti (i canali possono essere piu' dei pixel).
// u_flags: una colonna per riga (verde = inizio del periodo, giallo = periodo in cui si e' trasmesso), come le righe di JTDX.
const FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_uv;
out vec4 o;
uniform sampler2D u_tex;
uniform sampler2D u_lut;
uniform sampler2D u_flags;
uniform float u_head;
uniform float u_h;
uniform float u_frac;
uniform float u_rpp;
uniform vec2 u_px;
uniform vec2 u_tone;
float at(float x, float r) {
  return texture(u_tex, vec2(x, r)).r;
}
void main() {
  float py = v_uv.y * u_px.y * u_rpp;
  float coord = min(u_head - py + u_frac, u_head - 0.5);
  float r = fract(coord / u_h);
  float w = 1.0 / u_px.x;
  float v = 0.25 * (at(v_uv.x - 0.375 * w, r) + at(v_uv.x - 0.125 * w, r) + at(v_uv.x + 0.125 * w, r) + at(v_uv.x + 0.375 * w, r));
  float y = v * 255.0 / ${Y_PER_DB}.0 + (${Y_MIN}.0);
  float idx = clamp(trunc(u_tone.x * y + u_tone.y), 0.0, 254.0);
  vec3 c = texture(u_lut, vec2((idx + 0.5) / 256.0, 0.5)).rgb;
  vec2 f = texture(u_flags, vec2(0.5, r)).rg;
  c = mix(c, vec3(0.0, 1.0, 0.0), f.r);
  c = mix(c, vec3(1.0, 1.0, 0.0), f.g);
  o = vec4(c, 1.0);
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
  private uTone: WebGLUniformLocation | null = null;
  private flagTex: WebGLTexture | null = null;
  private tone: [number, number] = toneUniform(0, 0);
  private readonly clock = new RowClock();
  private raf = 0;
  private readonly ring = sharedRing(WATERFALL_WIDTH, WATERFALL_HEIGHT);

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
    this.uTone = gl.getUniformLocation(program, 'u_tone');

    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    this.tex = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.tex);
    // la storia gia' ricevuta (anello condiviso): un renderer nuovo dopo un ridimensionamento non parte vuoto
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, WATERFALL_WIDTH, WATERFALL_HEIGHT, 0, gl.RED, gl.UNSIGNED_BYTE, this.ring.data);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);

    this.lut = gl.createTexture();
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.lut);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, LUT_SIZE, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, jtdxPaletteLut(JTDX_PALETTES[DEFAULT_PALETTE]!));
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    this.flagTex = gl.createTexture();
    gl.activeTexture(gl.TEXTURE2);
    gl.bindTexture(gl.TEXTURE_2D, this.flagTex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RG8, 1, WATERFALL_HEIGHT, 0, gl.RG, gl.UNSIGNED_BYTE, this.ring.flags);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);

    gl.useProgram(program);
    gl.uniform1i(gl.getUniformLocation(program, 'u_tex'), 0);
    gl.uniform1i(gl.getUniformLocation(program, 'u_lut'), 1);
    gl.uniform1i(gl.getUniformLocation(program, 'u_flags'), 2);
    this.schedule();
  }

  pushRow(row: Uint8Array, flag: 0 | 1 | 2 = 0): void {
    const index = this.ring.push(row, flag);
    if (index < 0 || !this.program) return;
    const gl0 = this.gl;
    gl0.activeTexture(gl0.TEXTURE2);
    gl0.bindTexture(gl0.TEXTURE_2D, this.flagTex);
    gl0.texSubImage2D(gl0.TEXTURE_2D, 0, 0, index, 1, 1, gl0.RG, gl0.UNSIGNED_BYTE, this.ring.flags.subarray(index * 2, index * 2 + 2));
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
    gl.deleteTexture(this.flagTex);
    gl.deleteProgram(this.program);
    this.tex = this.lut = this.flagTex = this.program = null;
  }

  private schedule(): void {
    if (this.raf || !this.program) return;
    this.raf = requestAnimationFrame(() => {
      this.raf = 0;
      this.draw();
    });
  }

  setTone(gain: number, zero: number): void {
    this.tone = toneUniform(gain, zero);
    this.schedule();
  }

  setPalette(lut: Uint8Array): void {
    const gl = this.gl;
    if (!this.lut) return;
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.lut);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, LUT_SIZE, 1, gl.RGBA, gl.UNSIGNED_BYTE, lut);
    this.schedule();
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
    gl.uniform2f(this.uTone, this.tone[0], this.tone[1]);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    // la cascata scende di continuo: si ridisegna a ogni fotogramma
    this.schedule();
  }
}
