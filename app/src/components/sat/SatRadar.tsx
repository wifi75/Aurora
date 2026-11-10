import { useEffect, useRef } from 'react';
import { polarPoint } from '../../lib/satGeo';

interface Props {
  /** La traccia del passaggio: (azimut, elevazione) ogni 10 secondi. */
  path: readonly (readonly [number, number])[];
  /** Dove e' adesso il satellite (se e' sopra l'orizzonte). */
  now: { az: number; el: number } | null;
  /** Dove sta puntando il rotore. */
  rotor: { az: number; el: number } | null;
  size?: number;
}

/** Il radar del cielo: l'orizzonte sul bordo, lo zenit al centro, il nord in alto. Mostra il passaggio, il satellite e il rotore. */
export function SatRadar({ path, now, rotor, size = 230 }: Props) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const c = canvas.current;
    const ctx = c?.getContext('2d');
    if (!c || !ctx) return;
    const dpr = Math.min(2, window.devicePixelRatio || 1);
    c.width = Math.round(size * dpr);
    c.height = Math.round(size * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const cx = size / 2;
    const cy = size / 2;
    const R = size / 2 - 22;
    ctx.clearRect(0, 0, size, size);
    const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, R);
    g.addColorStop(0, 'rgba(40,70,120,0.55)');
    g.addColorStop(1, 'rgba(10,20,40,0.85)');
    ctx.fillStyle = g;
    ctx.beginPath();
    ctx.arc(cx, cy, R, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = 'rgba(255,255,255,0.28)';
    ctx.lineWidth = 1;
    for (const el of [0, 30, 60]) {
      ctx.beginPath();
      ctx.arc(cx, cy, ((90 - el) / 90) * R, 0, Math.PI * 2);
      ctx.stroke();
    }
    for (let az = 0; az < 360; az += 45) {
      const [x, y] = polarPoint(az, 0, R);
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(cx + x, cy + y);
      ctx.stroke();
    }
    ctx.fillStyle = 'rgba(255,255,255,0.85)';
    ctx.font = 'bold 12px system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (const [label, az] of [['N', 0], ['E', 90], ['S', 180], ['W', 270]] as const) {
      const [x, y] = polarPoint(az, 0, R + 11);
      ctx.fillText(label, cx + x, cy + y);
    }
    ctx.font = '10px system-ui, sans-serif';
    ctx.fillStyle = 'rgba(255,255,255,0.5)';
    ctx.textAlign = 'left';
    ctx.fillText('30°', cx + 3, cy - (R * 60) / 90 + 9);
    ctx.fillText('60°', cx + 3, cy - (R * 30) / 90 + 9);

    // il passaggio
    if (path.length > 1) {
      ctx.strokeStyle = '#ffd84d';
      ctx.lineWidth = 2.2;
      ctx.shadowColor = 'rgba(255,216,77,0.8)';
      ctx.shadowBlur = 6;
      ctx.beginPath();
      path.forEach(([az, el], i) => {
        const [x, y] = polarPoint(az, el, R);
        if (i === 0) ctx.moveTo(cx + x, cy + y);
        else ctx.lineTo(cx + x, cy + y);
      });
      ctx.stroke();
      ctx.shadowBlur = 0;
      const [sx, sy] = polarPoint(path[0]![0], path[0]![1], R);
      ctx.fillStyle = '#3ddc84';
      ctx.beginPath();
      ctx.arc(cx + sx, cy + sy, 3.5, 0, Math.PI * 2);
      ctx.fill();
      const last = path[path.length - 1]!;
      const [ex, ey] = polarPoint(last[0], last[1], R);
      ctx.fillStyle = '#ff6b6b';
      ctx.beginPath();
      ctx.arc(cx + ex, cy + ey, 3.5, 0, Math.PI * 2);
      ctx.fill();
    }
    // il rotore (un anello azzurro) e il satellite (il punto)
    if (rotor) {
      const [x, y] = polarPoint(rotor.az, rotor.el, R);
      ctx.strokeStyle = '#4dd2ff';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(cx + x, cy + y, 9, 0, Math.PI * 2);
      ctx.stroke();
    }
    if (now && now.el >= 0) {
      const [x, y] = polarPoint(now.az, now.el, R);
      ctx.fillStyle = '#fff';
      ctx.shadowColor = '#ffd84d';
      ctx.shadowBlur = 14;
      ctx.beginPath();
      ctx.arc(cx + x, cy + y, 5.5, 0, Math.PI * 2);
      ctx.fill();
      ctx.shadowBlur = 0;
    }
  }, [path, now, rotor, size]);

  return <canvas ref={canvas} className="sat-radar" style={{ width: size, height: size }} aria-label="radar" />;
}
