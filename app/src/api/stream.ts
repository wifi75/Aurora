import { EventSchema, type AuroraEvent } from './schemas';

const FRAME_SPECTRUM = 0x01;
const HEADER_BYTES = 17;
const MAX_BINS = 4096;

export interface SpectrumFrame {
  seq: number;
  fMinHz: number;
  fMaxHz: number;
  bins: Uint8Array;
}

/** Frame binario del core: `[0x01][seq u64][f_min u32][f_max u32][bins...]`, tutto big-endian. */
export function decodeSpectrumFrame(buffer: ArrayBuffer): SpectrumFrame | null {
  if (buffer.byteLength <= HEADER_BYTES) return null;
  const view = new DataView(buffer);
  if (view.getUint8(0) !== FRAME_SPECTRUM) return null;
  const binCount = buffer.byteLength - HEADER_BYTES;
  const fMinHz = view.getUint32(9);
  const fMaxHz = view.getUint32(13);
  if (binCount > MAX_BINS || fMinHz >= fMaxHz) return null;
  return {
    seq: Number(view.getBigUint64(1)),
    fMinHz,
    fMaxHz,
    bins: new Uint8Array(buffer.slice(HEADER_BYTES)),
  };
}

export type ConnectionState = 'connecting' | 'open' | 'closed';

export interface StreamHandlers {
  onEvent(event: AuroraEvent): void;
  onSpectrum(frame: SpectrumFrame): void;
  onState(state: ConnectionState): void;
}

const RETRY_MIN_MS = 500;
const RETRY_MAX_MS = 8000;

function streamUrl(): string {
  const scheme = location.protocol === 'https:' ? 'wss' : 'ws';
  return `${scheme}://${location.host}/api/v1/stream`;
}

/** Apre il flusso e lo riapre da solo, con attesa crescente, finche' non si chiama `close()`. */
export function openStream(handlers: StreamHandlers): { close(): void } {
  let socket: WebSocket | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let delay = RETRY_MIN_MS;
  let stopped = false;

  const connect = () => {
    if (stopped) return;
    handlers.onState('connecting');
    const ws = new WebSocket(streamUrl());
    ws.binaryType = 'arraybuffer';
    socket = ws;
    ws.onopen = () => {
      delay = RETRY_MIN_MS;
      handlers.onState('open');
    };
    ws.onmessage = (message: MessageEvent) => {
      if (typeof message.data === 'string') {
        try {
          const parsed = EventSchema.safeParse(JSON.parse(message.data));
          if (parsed.success) handlers.onEvent(parsed.data);
        } catch {
          // frame di testo non JSON: ignorato
        }
      } else if (message.data instanceof ArrayBuffer) {
        const frame = decodeSpectrumFrame(message.data);
        if (frame) handlers.onSpectrum(frame);
      }
    };
    ws.onclose = () => {
      handlers.onState('closed');
      if (stopped) return;
      timer = setTimeout(connect, delay);
      delay = Math.min(delay * 2, RETRY_MAX_MS);
    };
    ws.onerror = () => ws.close();
  };

  connect();
  return {
    close() {
      stopped = true;
      if (timer) clearTimeout(timer);
      socket?.close();
    },
  };
}
