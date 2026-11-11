import { describe, expect, it } from 'vitest';
import { parseSpotSplit } from './spotSplit';

const at = (comment: string | null, khz = 14025, mode: string | null = 'CW') => parseSpotSplit(comment, khz, mode);

describe('parseSpotSplit', () => {
  it('legge «UP N» con o senza spazio, maiuscolo o minuscolo, in qualunque punto del commento', () => {
    for (const c of ['UP 5', 'UP5', 'up5', 'up 5', 'Up 5', 'pse UP 5 tnx', 'UP 5K', 'UP 5KHZ', 'up 5 khz', 'UP +5', 'QRZ? UP5 !']) {
      expect(at(c), c).toEqual({ kind: 'up', offsetKhz: 5, txKhz: 14030, label: 'UP 5' });
    }
  });

  it('legge il numero prima: «5 UP», «5UP», «10up», «10UP»', () => {
    expect(at('5 UP')?.offsetKhz).toBe(5);
    expect(at('5UP')?.offsetKhz).toBe(5);
    expect(at('5up')?.offsetKhz).toBe(5);
    for (const c of ['10up', '10UP', '10 up', 'CQ CQ 10up QRZ', 'tnx 10 UP pse']) {
      expect(at(c), c).toEqual({ kind: 'up', offsetKhz: 10, txKhz: 14035, label: 'UP 10' });
    }
  });

  it('DN e DOWN, prima o dopo il numero, spostano sotto', () => {
    for (const c of ['DN 2', 'dn2', 'DOWN 2', 'down 2', '2 DN', '2DN', '2 DOWN', '2down', 'DWN 2']) {
      expect(at(c), c).toEqual({ kind: 'down', offsetKhz: -2, txKhz: 14023, label: 'DN 2' });
    }
  });

  it('un intervallo vale il suo centro e l’etichetta lo ripete', () => {
    expect(at('UP 1-3')).toEqual({ kind: 'up', offsetKhz: 2, txKhz: 14027, label: 'UP 1-3' });
    expect(at('UP 1/3')?.offsetKhz).toBe(2);
    expect(at('1-3 UP')?.offsetKhz).toBe(2);
    expect(at('up 5-10')).toEqual({ kind: 'up', offsetKhz: 7.5, txKhz: 14032.5, label: 'UP 5-10' });
    expect(at('UP 10-5')?.offsetKhz).toBe(7.5);
    expect(at('DN 1-3')?.offsetKhz).toBe(-2);
  });

  it('decimali con punto o virgola', () => {
    expect(at('UP 1.5')?.offsetKhz).toBe(1.5);
    expect(at('UP 1,5')).toEqual({ kind: 'up', offsetKhz: 1.5, txKhz: 14026.5, label: 'UP 1.5' });
  });

  it('«UP» o «DN» da soli valgono 1 kHz', () => {
    expect(at('UP')).toEqual({ kind: 'up', offsetKhz: 1, txKhz: 14026, label: 'UP 1' });
    expect(at('listening up')?.offsetKhz).toBe(1);
    expect(at('DN')?.offsetKhz).toBe(-1);
  });

  it('QSX: frequenza intera, in MHz o solo le ultime cifre', () => {
    expect(at('QSX 14.030')).toEqual({ kind: 'qsx', offsetKhz: 5, txKhz: 14030, label: 'QSX 14.030' });
    expect(at('qsx 14030')?.txKhz).toBe(14030);
    expect(at('QSX 030')?.txKhz).toBe(14030);
    expect(at('QSX: 14.0305')).toBeNull();
    expect(at('QSX 7.165', 7090, 'SSB')).toEqual({ kind: 'qsx', offsetKhz: 75, txKhz: 7165, label: 'QSX 7.165' });
    expect(at('QSX 205', 14195, 'SSB')?.txKhz).toBe(14205);
    // il MHz accanto, se piu' vicino
    expect(at('QSX 002', 28998, 'SSB')?.txKhz).toBe(29002);
    expect(at('QSX 101', 7099, 'SSB')?.txKhz).toBe(7101);
    // fuori dalla banda dello spot
    expect(at('QSX 995', 14002)).toBeNull();
    expect(at('QSX 7.030')).toBeNull();
  });

  it('«UP 14.030» o «UP 14030» nella banda dello spot sono la frequenza dove ascolta', () => {
    expect(at('UP 14.030')).toEqual({ kind: 'qsx', offsetKhz: 5, txKhz: 14030, label: 'QSX 14.030' });
    expect(at('UP 14030')?.txKhz).toBe(14030);
  });

  it('i numeri che non sono spostamenti non si indovinano', () => {
    expect(at('599 UP')).toBeNull();
    expect(at('UP 599')).toBeNull();
    expect(at('UP 60')).toBeNull();
    expect(at('UP 0')).toBeNull();
    expect(at('1234Z')).toBeNull();
  });

  it('niente UP dentro le parole o nei locatori, ma il primo spostamento vero vale', () => {
    expect(at('SETUP')).toBeNull();
    expect(at('UPLOAD LOG')).toBeNull();
    expect(at('SHUTDOWN')).toBeNull();
    expect(at('TNX QSO 73')).toBeNull();
    expect(at('DN70 UP 2')?.offsetKhz).toBe(2);
    expect(at('')).toBeNull();
    expect(at(null)).toBeNull();
    expect(parseSpotSplit('UP 5', 0, 'CW')).toBeNull();
  });

  it('negli spot FT8, FT4 e degli altri modi WSJT niente Split DX; con CW, SSB, RTTY o modo ignoto si', () => {
    for (const mode of ['FT8', 'ft4', 'FST4', 'JT65', 'Q65', 'MSK144']) expect(at('UP 5', 14025, mode), mode).toBeNull();
    for (const mode of ['CW', 'SSB', 'RTTY', 'PSK31', null]) expect(at('UP 5', 14025, mode)?.offsetKhz, String(mode)).toBe(5);
  });
});
