import type { WaterfallBackend } from './backend';
import { LUT_SIZE, NOTTE_RADIO_STOPS, buildLut } from './colormap';
import { RowRing } from './ring';
import { WATERFALL_HEIGHT, WATERFALL_WIDTH } from './webgl2';

const SHADER = /* wgsl */ `
struct U { head: f32, h: f32, pad0: f32, pad1: f32 };
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var lut: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

struct VOut { @builtin(position) pos: vec4f, @location(0) uv: vec2f };

@vertex fn vs(@builtin(vertex_index) i: u32) -> VOut {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  var o: VOut;
  o.pos = vec4f(p * 2.0 - 1.0, 0.0, 1.0);
  o.uv = vec2f(p.x, 1.0 - p.y);
  return o;
}

@fragment fn fs(in: VOut) -> @location(0) vec4f {
  let r = fract((u.head - 0.5 - in.uv.y * u.h) / u.h);
  let v = textureSampleLevel(tex, samp, vec2f(in.uv.x, r), 0.0).r;
  let c = textureSampleLevel(lut, samp, vec2f(v * 0.99609375 + 0.001953125, 0.5), 0.0).rgb;
  return vec4f(c, 1.0);
}`;

export class WebGpuWaterfall implements WaterfallBackend {
  readonly kind = 'webgpu' as const;
  private canvas!: HTMLCanvasElement;
  private device!: GPUDevice;
  private context!: GPUCanvasContext;
  private pipeline!: GPURenderPipeline;
  private bindGroup!: GPUBindGroup;
  private texture!: GPUTexture;
  private lutTexture!: GPUTexture;
  private uniforms!: GPUBuffer;
  private raf = 0;
  private alive = false;
  private readonly ring = new RowRing(WATERFALL_WIDTH, WATERFALL_HEIGHT);

  async init(canvas: HTMLCanvasElement): Promise<void> {
    if (!navigator.gpu) throw new Error('WebGPU non disponibile');
    // adattatore e dispositivo prima del contesto: se falliscono, il canvas resta libero per WebGL2
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) throw new Error('nessun adattatore WebGPU');
    const device = await adapter.requestDevice();
    const context = canvas.getContext('webgpu');
    if (!context) throw new Error('contesto WebGPU non disponibile');
    const format = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format, alphaMode: 'opaque' });

    this.canvas = canvas;
    this.device = device;
    this.context = context;

    this.texture = device.createTexture({
      size: [WATERFALL_WIDTH, WATERFALL_HEIGHT],
      format: 'r8unorm',
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    });
    this.lutTexture = device.createTexture({
      size: [LUT_SIZE, 1],
      format: 'rgba8unorm',
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    });
    const lut = buildLut(NOTTE_RADIO_STOPS);
    device.queue.writeTexture({ texture: this.lutTexture }, lut.buffer as ArrayBuffer, { bytesPerRow: LUT_SIZE * 4 }, [LUT_SIZE, 1]);

    this.uniforms = device.createBuffer({ size: 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    const sampler = device.createSampler({
      magFilter: 'linear',
      minFilter: 'linear',
      addressModeU: 'clamp-to-edge',
      addressModeV: 'repeat',
    });
    const module = device.createShaderModule({ code: SHADER });
    this.pipeline = device.createRenderPipeline({
      layout: 'auto',
      vertex: { module, entryPoint: 'vs' },
      fragment: { module, entryPoint: 'fs', targets: [{ format }] },
      primitive: { topology: 'triangle-list' },
    });
    this.bindGroup = device.createBindGroup({
      layout: this.pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: this.uniforms } },
        { binding: 1, resource: this.texture.createView() },
        { binding: 2, resource: this.lutTexture.createView() },
        { binding: 3, resource: sampler },
      ],
    });
    this.alive = true;
    this.schedule();
  }

  pushRow(row: Uint8Array): void {
    const index = this.ring.push(row);
    if (index < 0 || !this.alive) return;
    const slice = this.ring.data.slice(index * WATERFALL_WIDTH, (index + 1) * WATERFALL_WIDTH);
    this.device.queue.writeTexture(
      { texture: this.texture, origin: [0, index, 0] },
      slice.buffer as ArrayBuffer,
      { bytesPerRow: WATERFALL_WIDTH },
      [WATERFALL_WIDTH, 1],
    );
    this.schedule();
  }

  resize(widthPx: number, heightPx: number): void {
    this.canvas.width = Math.max(1, Math.floor(widthPx));
    this.canvas.height = Math.max(1, Math.floor(heightPx));
    this.schedule();
  }

  destroy(): void {
    this.alive = false;
    if (this.raf) cancelAnimationFrame(this.raf);
    this.raf = 0;
    this.texture?.destroy();
    this.lutTexture?.destroy();
    this.uniforms?.destroy();
    this.device?.destroy();
  }

  private schedule(): void {
    if (this.raf || !this.alive) return;
    this.raf = requestAnimationFrame(() => {
      this.raf = 0;
      this.draw();
    });
  }

  private draw(): void {
    if (!this.alive) return;
    this.device.queue.writeBuffer(this.uniforms, 0, new Float32Array([this.ring.head, WATERFALL_HEIGHT, 0, 0]));
    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: this.context.getCurrentTexture().createView(),
          loadOp: 'clear',
          storeOp: 'store',
          clearValue: { r: 0, g: 0, b: 0, a: 1 },
        },
      ],
    });
    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.bindGroup);
    pass.draw(3);
    pass.end();
    this.device.queue.submit([encoder.finish()]);
  }
}
