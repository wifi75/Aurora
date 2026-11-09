import * as L from 'leaflet';

/** Un punto della mappa di calore: posizione e peso (0–1). */
export interface HeatPoint {
  lat: number;
  lon: number;
  weight: number;
}

// colori dal freddo al caldo, come le mappe di calore dei programmi radioamatoriali (blu, azzurro, verde, giallo, rosso)
const STOPS: Array<[number, string]> = [
  [0.4, '#0000ff'],
  [0.6, '#00ffff'],
  [0.7, '#00ff00'],
  [0.8, '#ffff00'],
  [1, '#ff0000'],
];

function palette(): Uint8ClampedArray {
  const c = document.createElement('canvas');
  c.width = 1;
  c.height = 256;
  const ctx = c.getContext('2d')!;
  const g = ctx.createLinearGradient(0, 0, 0, 256);
  for (const [at, color] of STOPS) g.addColorStop(at, color);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, 1, 256);
  return ctx.getImageData(0, 0, 1, 256).data;
}

/**
 * Mappa di calore su una tela sopra la mappa: ogni punto lascia una macchia sfumata, le macchie vicine si sommano e il totale
 * diventa un colore (blu dove c'e' poco, rosso dove c'e' tanto). Si ridisegna a ogni spostamento o zoom.
 */
export class HeatLayer extends L.Layer {
  private canvas: HTMLCanvasElement | null = null;
  private points: HeatPoint[] = [];
  private colors: Uint8ClampedArray | null = null;

  constructor(private radius = 22, private blur = 16, private opacity = 0.75) {
    super();
  }

  setPoints(points: HeatPoint[]): void {
    this.points = points;
    this.redraw();
  }

  override onAdd(map: L.Map): this {
    const canvas = L.DomUtil.create('canvas', 'dxmap-heat leaflet-zoom-hide') as HTMLCanvasElement;
    canvas.style.position = 'absolute';
    canvas.style.pointerEvents = 'none';
    canvas.style.opacity = String(this.opacity);
    this.canvas = canvas;
    map.getPanes().overlayPane.appendChild(canvas);
    map.on('moveend zoomend resize', this.redraw, this);
    this.redraw();
    return this;
  }

  override onRemove(map: L.Map): this {
    map.off('moveend zoomend resize', this.redraw, this);
    this.canvas?.remove();
    this.canvas = null;
    return this;
  }

  private redraw(): void {
    const map = this._map as L.Map | undefined;
    const canvas = this.canvas;
    if (!map || !canvas) return;
    const size = map.getSize();
    canvas.width = size.x;
    canvas.height = size.y;
    L.DomUtil.setPosition(canvas, map.containerPointToLayerPoint([0, 0]));
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.clearRect(0, 0, size.x, size.y);
    // mappa nascosta o non ancora misurata (pannello in una pagina nascosta): niente da disegnare. Con misura 0 `getImageData`
    // andava in errore e la pagina restava vuota (2026-11-09)
    if (this.points.length === 0 || size.x < 1 || size.y < 1) return;
    // macchie in scala di grigi (solo trasparenza), poi la trasparenza diventa colore
    const r = this.radius + this.blur;
    const stamp = document.createElement('canvas');
    stamp.width = stamp.height = r * 2;
    const sctx = stamp.getContext('2d')!;
    const grad = sctx.createRadialGradient(r, r, 0, r, r, r);
    grad.addColorStop(0, 'rgba(0,0,0,1)');
    grad.addColorStop(this.radius / r, 'rgba(0,0,0,0.5)');
    grad.addColorStop(1, 'rgba(0,0,0,0)');
    sctx.fillStyle = grad;
    sctx.fillRect(0, 0, r * 2, r * 2);
    for (const p of this.points) {
      const pt = map.latLngToContainerPoint([p.lat, p.lon]);
      if (pt.x < -r || pt.y < -r || pt.x > size.x + r || pt.y > size.y + r) continue;
      ctx.globalAlpha = Math.max(0.05, Math.min(1, p.weight));
      ctx.drawImage(stamp, pt.x - r, pt.y - r);
    }
    ctx.globalAlpha = 1;
    this.colors ??= palette();
    const img = ctx.getImageData(0, 0, size.x, size.y);
    const d = img.data;
    for (let i = 3; i < d.length; i += 4) {
      const a = d[i]!;
      if (a === 0) continue;
      const j = a * 4;
      d[i - 3] = this.colors[j]!;
      d[i - 2] = this.colors[j + 1]!;
      d[i - 1] = this.colors[j + 2]!;
    }
    ctx.putImageData(img, 0, 0);
  }
}
