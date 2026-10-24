import type { WaterfallBackend } from './backend';
import { LUT_SIZE, NOTTE_RADIO_STOPS, buildLut } from './colormap';
import { RowRing } from './ring';

export const WATERFALL_WIDTH = 1024;
export const WATERFALL_HEIGHT = 512;

const VERTEX = `#version 300 es
out vec2 v_uv;
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  v_uv = vec2(p.x, 1.0 - p.y);
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}`;

// La riga piu' recente sta in alto: lo scorrimento e' solo un offset sull'anello, nessuna ricopiatura.
const FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_uv;
out vec4 o;
uniform sampler2D u_tex;
uniform sampler2D u_lut;
uniform float u_head;
uniform float u_h;
void main() {
  float r = fract((u_head - 0.5 - v_uv.y * u_h) / u_h);
  float v = texture(u_tex, vec2(v_uv.x, r)).r;
  o = vec4(texture(u_lut, vec2(v * 0.99609375 + 0.001953125, 0.5)).rgb, 1.0);
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
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, LUT_SIZE, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, buildLut(NOTTE_RADIO_STOPS));
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

  private draw(): void {
    const gl = this.gl;
    if (!this.program) return;
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.useProgram(this.program);
    gl.uniform1f(this.uHead, this.ring.head);
    gl.uniform1f(this.uH, WATERFALL_HEIGHT);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
}
