/**
 * Il grafico dello spettro sotto la cascata, come `CPlotter::DrawOverlay` e `CPlotter::draw` di JTDX: sfondo sfumato dal blu scuro
 * al nero, 7 divisioni orizzontali e le verticali a passo di frequenza, curva verde.
 */

export const VERT_DIVS = 7;

/** Passo delle linee verticali (Hz) in funzione dell'ampiezza mostrata (`DrawOverlay`). */
export function freqPerDiv(spanHz: number): number {
  if (spanHz > 2500) return 500;
  if (spanHz > 1000) return 200;
  if (spanHz > 500) return 100;
  if (spanHz > 250) return 50;
  if (spanHz > 100) return 20;
  return 10;
}

export interface PlotColors {
  top: string;
  bottom: string;
  grid: string;
  curve: string;
}

/** I colori di JTDX: fondo darkBlue -> nero, griglia bianca a puntini, curva verde. */
export const JTDX_PLOT_COLORS: PlotColors = { top: '#000080', bottom: '#000000', grid: 'rgba(255,255,255,0.55)', curve: '#00ff00' };

/** Disegna lo sfondo, la griglia e la curva (ordinate come frazioni di altezza dall'alto, una per pixel di cascata). */
export function drawSpectrum(
  canvas: HTMLCanvasElement,
  curve: Float32Array | null,
  fromHz: number,
  toHz: number,
  colors: PlotColors = JTDX_PLOT_COLORS,
): void {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  const w = canvas.width;
  const h = canvas.height;
  if (w < 2 || h < 2) return;
  const gradient = ctx.createLinearGradient(0, 0, 0, h);
  gradient.addColorStop(0, colors.top);
  gradient.addColorStop(1, colors.bottom);
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, w, h);

  ctx.strokeStyle = colors.grid;
  ctx.lineWidth = 1;
  ctx.setLineDash([1, 3]);
  const step = freqPerDiv(toHz - fromHz);
  ctx.beginPath();
  for (let f = Math.ceil(fromHz / step) * step; f < toHz; f += step) {
    const x = Math.round(((f - fromHz) / (toHz - fromHz)) * w) + 0.5;
    ctx.moveTo(x, 0);
    ctx.lineTo(x, h);
  }
  for (let i = 1; i < VERT_DIVS; i++) {
    const y = Math.round((i * h) / VERT_DIVS) + 0.5;
    ctx.moveTo(0, y);
    ctx.lineTo(w, y);
  }
  ctx.stroke();
  ctx.setLineDash([]);

  if (!curve || curve.length < 2) return;
  ctx.strokeStyle = colors.curve;
  ctx.lineWidth = Math.max(1, Math.round(h / 120));
  ctx.beginPath();
  for (let i = 0; i < curve.length; i++) {
    const x = (i / (curve.length - 1)) * w;
    const y = Math.min(h, Math.max(0, curve[i]! * h));
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();
}
