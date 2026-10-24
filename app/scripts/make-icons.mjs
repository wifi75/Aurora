// Genera le icone PNG dell'app (onda ciano su blu notte) senza dipendenze: node scripts/make-icons.mjs
import { deflateSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';

const BG = [11, 14, 20];
const FG = [63, 214, 200];

function crc32(buf) {
  let c;
  let crc = 0xffffffff;
  for (let n = 0; n < buf.length; n++) {
    c = (crc ^ buf[n]) & 0xff;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    crc = (crc >>> 8) ^ c;
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  out.write(type, 4, 'ascii');
  data.copy(out, 8);
  out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length);
  return out;
}

function png(size) {
  const raw = Buffer.alloc((size * 4 + 1) * size);
  const half = (size * 0.075) / 1; // mezzo spessore del tratto
  const samples = 3;
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    for (let x = 0; x < size; x++) {
      let cover = 0;
      for (let sy = 0; sy < samples; sy++) {
        for (let sx = 0; sx < samples; sx++) {
          const px = (x + (sx + 0.5) / samples) / size;
          const py = (y + (sy + 0.5) / samples) / size;
          // onda sinusoidale (area sicura al centro, adatta anche alle icone "maskable")
          const cx = 0.2 + px * 0.6;
          const f = 0.5 + 0.17 * Math.sin((cx - 0.2) / 0.6 * Math.PI * 3);
          const slope = 0.17 * Math.cos((cx - 0.2) / 0.6 * Math.PI * 3) * (Math.PI * 3) / 0.6 * 0.6;
          const inRange = px >= 0.2 && px <= 0.8;
          const dist = Math.abs(py - f) / Math.sqrt(1 + slope * slope);
          if (inRange && dist * size < half) cover++;
        }
      }
      const a = cover / (samples * samples);
      const i = y * (size * 4 + 1) + 1 + x * 4;
      for (let ch = 0; ch < 3; ch++) raw[i + ch] = Math.round(BG[ch] + (FG[ch] - BG[ch]) * a);
      raw[i + 3] = 255;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

mkdirSync('public', { recursive: true });
for (const [name, size] of [
  ['icon-192.png', 192],
  ['icon-512.png', 512],
  ['apple-touch-icon.png', 180],
]) {
  writeFileSync(`public/${name}`, png(size));
  console.log(`scritto public/${name} (${size}x${size})`);
}
