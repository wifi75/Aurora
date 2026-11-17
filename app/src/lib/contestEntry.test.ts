import { describe, expect, it } from 'vitest';
import { newTextContest, type TextContest } from '../api/contestTesto';
import { autoExchange, callMultipliers, contestSuggestions, exchangeRcvdOf, multSummary, previousQsos } from './contestEntry';
import { checkContestDates, contestWindowStatus, lastFullWeekend, staleWindowProposal, validContestDate, windowNoticeInfo, windowProposals } from './textContest';

const c = (p: Partial<TextContest> = {}): TextContest => ({ ...newTextContest(), name: 'Prova', ...p });
const day = (ms: number) => new Date(ms).toISOString().slice(0, 10);

describe('ultimo fine settimana completo', () => {
  it('sabato e domenica dentro il mese', () => {
    // l'esempio del 2020: CQ WW CW il 28 e 29 novembre; SSB il 24 e 25 ottobre (il 31 e' sabato ma la domenica e' a novembre)
    expect([day(lastFullWeekend(2020, 10).from), day(lastFullWeekend(2020, 10).to)]).toEqual(['2020-11-28', '2020-11-29']);
    expect([day(lastFullWeekend(2020, 9).from), day(lastFullWeekend(2020, 9).to)]).toEqual(['2020-10-24', '2020-10-25']);
    expect([day(lastFullWeekend(2026, 9).from), day(lastFullWeekend(2026, 9).to)]).toEqual(['2026-10-24', '2026-10-25']);
    expect([day(lastFullWeekend(2026, 10).from), day(lastFullWeekend(2026, 10).to)]).toEqual(['2026-11-28', '2026-11-29']);
    expect([day(lastFullWeekend(2026, 2).from), day(lastFullWeekend(2026, 2).to)]).toEqual(['2026-03-28', '2026-03-29']);
    // maggio 2026 finisce di domenica
    expect([day(lastFullWeekend(2026, 4).from), day(lastFullWeekend(2026, 4).to)]).toEqual(['2026-05-30', '2026-05-31']);
  });

  it('inizia a 00:00:00 e finisce a 23:59:59 UTC', () => {
    const w = lastFullWeekend(2026, 10);
    expect(new Date(w.from).toISOString()).toBe('2026-11-28T00:00:00.000Z');
    expect(new Date(w.to).toISOString()).toBe('2026-11-29T23:59:59.000Z');
  });
});

describe('proposte per i contest ricorrenti', () => {
  it('riconosce il CQ WW DX e il WPX dal nome', () => {
    expect(windowProposals('CQ WW DX Contest', 2026).map((p) => [p.tag, p.start, p.end])).toEqual([
      ['SSB', '20261024000000', '20261025235959'],
      ['CW', '20261128000000', '20261129235959'],
    ]);
    expect(windowProposals('CQ WW DX CW', 2026).map((p) => p.start)).toEqual(['20261128000000']);
    expect(windowProposals('CQ WW WPX SSB Contest', 2026).map((p) => p.start)).toEqual(['20260328000000']);
    expect(windowProposals('CQ WW WPX CW Contest', 2026).map((p) => p.start)).toEqual(['20260530000000']);
  });

  it('non inventa quello che non conosce', () => {
    expect(windowProposals('CQ WW RTTY DX', 2026)).toEqual([]);
    expect(windowProposals('CQ WW 160 meter contest', 2026)).toEqual([]);
    expect(windowProposals('CQ WPX RTTY Contest', 2026)).toEqual([]);
    expect(windowProposals('Contest della mia sezione', 2026)).toEqual([]);
  });

  it('precompila solo con date di un anno passato e una sola edizione', () => {
    const now = Date.UTC(2026, 10, 16, 12);
    expect(staleWindowProposal(c({ name: 'CQ WW WPX CW Contest', start: '20170325000000', end: '20170326235959' }), now)?.start).toBe('20260530000000');
    expect(staleWindowProposal(c({ name: 'CQ WW WPX CW Contest', start: '20260530000000', end: '20260531235959' }), now)).toBeNull();
    expect(staleWindowProposal(c({ name: 'CQ WW DX Contest', start: '20161028072401', end: '20161028235959' }), now)).toBeNull();
    expect(staleWindowProposal(c({ name: 'Altro', start: '20150101000000', end: '20150102000000' }), now)).toBeNull();
  });
});

describe('Japan International DX', () => {
  it('2o fine settimana completo, sabato 07:00 domenica 13:00 UTC', () => {
    const iso = (s: string) => new Date(Date.UTC(+s.slice(0, 4), +s.slice(4, 6) - 1, +s.slice(6, 8), +s.slice(8, 10), +s.slice(10, 12), +s.slice(12, 14))).toISOString();
    const p = (n: string, y: number) => windowProposals(n, y).map((x) => [iso(x.start), iso(x.end)]);
    expect(p('Japan International DX CW Contest', 2026)).toEqual([['2026-04-11T07:00:00.000Z', '2026-04-12T13:00:00.000Z']]);
    expect(p('Japan International DX SSB Contest', 2026)).toEqual([['2026-11-14T07:00:00.000Z', '2026-11-15T13:00:00.000Z']]);
    expect(p('Japan International DX CW Contest', 2027)).toEqual([['2027-04-10T07:00:00.000Z', '2027-04-11T13:00:00.000Z']]);
    expect(p('Japan International DX SSB Contest', 2027)).toEqual([['2027-11-13T07:00:00.000Z', '2027-11-14T13:00:00.000Z']]);
  });
});

describe('controllo delle date', () => {
  it('rifiuta date inesistenti e fine prima dell inizio', () => {
    expect(validContestDate('20261128000001')).toBe(true);
    expect(validContestDate('20261131000000')).toBe(false);
    expect(validContestDate('20261128250000')).toBe(false);
    expect(checkContestDates('', '')).toBe('ok');
    expect(checkContestDates('20261128000001', '')).toBe('ok');
    expect(checkContestDates('20261128000001', '20261129235959')).toBe('ok');
    expect(checkContestDates('20261129235959', '20261128000001')).toBe('order');
    expect(checkContestDates('20261128000001', '20261128000001')).toBe('order');
    expect(checkContestDates('2026', '')).toBe('date');
  });
});

describe('stato della finestra', () => {
  it('distingue senza date, prima, dentro e scaduta', () => {
    const k = c({ start: '20261128000001', end: '20261129235959' });
    expect(contestWindowStatus(c(), Date.UTC(2026, 10, 16)).status).toBe('none');
    expect(contestWindowStatus(k, Date.UTC(2026, 10, 16)).status).toBe('before');
    expect(contestWindowStatus(k, Date.UTC(2026, 10, 28, 12)).status).toBe('active');
    expect(contestWindowStatus(k, Date.UTC(2026, 11, 2)).status).toBe('expired');
  });

  it('il file del CQ WW DX con data del 2016 e scaduto a novembre 2026', () => {
    expect(contestWindowStatus(c({ start: '20161028072401', end: '20161028235959' }), Date.UTC(2026, 10, 16)).status).toBe('expired');
  });
});

describe('avviso della finestra', () => {
  it('tre testi diversi: non ancora iniziato (con la data), scaduto, non impostato', () => {
    const k = c({ start: '20261212000000', end: '20261213235959' });
    expect(windowNoticeInfo(k, Date.UTC(2026, 10, 17))).toEqual({ key: 'ssbc.windowBefore', when: '2026-12-12 00:00' });
    expect(windowNoticeInfo(k, Date.UTC(2026, 11, 20)).key).toBe('ssbc.windowExpired');
    expect(windowNoticeInfo(c(), Date.UTC(2026, 10, 17)).key).toBe('ssbc.windowOff');
  });
});

describe('suggerimenti e scambio dal nominativo', () => {
  const info = { prefix: 'I', cq_zone: 15, itu_zone: 28 };
  it('propone solo quello che il contest ha acceso', () => {
    expect(contestSuggestions(c({ suggest_cq_zone: true, suggest_dxcc_pfx: true }), info)).toEqual([
      { kind: 'cq', value: '15' },
      { kind: 'dxcc', value: 'I' },
    ]);
    expect(contestSuggestions(c(), info)).toEqual([]);
    expect(contestSuggestions(c({ suggest_cq_zone: true }), null)).toEqual([]);
  });

  it('precompila la zona secondo il tipo di numero, non il seriale', () => {
    expect(autoExchange(c({ grab_number: true, grab_number_type: 2 }), info)).toBe('15');
    expect(autoExchange(c({ grab_number: true, grab_number_type: 1 }), info)).toBe('28');
    expect(autoExchange(c({ grab_number: true, grab_number_type: 0 }), info)).toBeNull();
    expect(autoExchange(c({ grab_number: false, grab_number_type: 2 }), info)).toBeNull();
    expect(autoExchange(c({ grab_number: true, grab_number_type: -1 }), info)).toBeNull();
  });
});

describe('QSO precedenti', () => {
  const q = (call: string, band: string, mode: string, time: string, comment = '') => ({ call, band, mode, date: '2026-10-24', time, comment });
  const all = [
    q('IU3ABC', '20m', 'SSB', '10:00', 'NRS 001 NRR 15'),
    q('IU3ABC', '20m', 'SSB', '12:30', 'NRS 004 NRR 16'),
    q('IU3ABC', '40m', 'CW', '09:15'),
    q('DL1XYZ', '15m', 'SSB', '11:00', 'NRS 002 NRR 14'),
    q('IU3ABC', '12m', 'SSB', '13:00', 'NRR 15'), // WARC: non e' una banda da contest
  ];

  it('una riga per banda con l ultimo QSO e - dove non c e', () => {
    const rows = previousQsos(all, 'iu3abc', () => true);
    expect(rows.map((r) => r.band)).toEqual(['160m', '80m', '40m', '20m', '15m', '10m']);
    expect(rows.find((r) => r.band === '20m')?.qso).toEqual({ mode: 'SSB', time_ms: Date.UTC(2026, 9, 24, 12, 30), nr: '16' });
    expect(rows.find((r) => r.band === '40m')?.qso?.nr).toBe('');
    expect(rows.find((r) => r.band === '160m')?.qso).toBeNull();
  });

  it('conta solo i QSO del contest e non per nominativi troppo corti', () => {
    expect(previousQsos(all, 'IU3ABC', (x) => x.band === '40m').filter((r) => r.qso).map((r) => r.band)).toEqual(['40m']);
    expect(previousQsos(all, 'IU', () => true).every((r) => r.qso === null)).toBe(true);
  });

  it('legge lo scambio ricevuto dal commento', () => {
    expect(exchangeRcvdOf('nota NRS 001 NRR 015')).toBe('015');
    expect(exchangeRcvdOf(undefined)).toBe('');
  });
});

describe('moltiplicatori per tipo', () => {
  const s = {
    tipi: ['cq', 'dxcc'],
    bande: [{ band: '20m' }, { band: '40m' }],
    griglia: [
      { tipo: 'cq', valore: '15', bande: ['20m', '40m'] },
      { tipo: 'cq', valore: '14', bande: ['20m'] },
      { tipo: 'dxcc', valore: 'DL', bande: ['20m'] },
    ],
    mancanti: [{ tipo: 'cq', per_banda: true, totali: 40, valori: [] }],
  };

  it('conta lavorati, totali e per banda', () => {
    expect(multSummary(s)).toEqual([
      { tipo: 'cq', worked: 2, total: 40, perBand: { '20m': 2, '40m': 1 } },
      { tipo: 'dxcc', worked: 1, total: null, perBand: { '20m': 1, '40m': 0 } },
    ]);
  });

  it('dice per il nominativo digitato cosa e nuovo e cosa gia lavorato', () => {
    const info = { prefix: 'I', cq_zone: 15, itu_zone: 28, continent: 'EU' };
    const g = [
      { tipo: 'cq', valore: '15', bande: ['20m'] },
      { tipo: 'dxcc', valore: 'I', bande: ['40m'] },
    ];
    expect(callMultipliers(['cq', 'dxcc'], info, [], g, '20m')).toEqual([
      { tipo: 'cq', value: '15', fresh: false },
      { tipo: 'dxcc', value: 'I', fresh: true }, // lavorato su 40 m, non su 20 m
    ]);
    expect(callMultipliers(['cq'], info, [], [], '20m')).toEqual([{ tipo: 'cq', value: '15', fresh: true }]);
    expect(callMultipliers(['wpx'], info, ['wpx:IU3'])).toEqual([{ tipo: 'wpx', value: 'IU3', fresh: true }]);
    expect(callMultipliers(['cq'], info, ['cq:015'])).toEqual([{ tipo: 'cq', value: '15', fresh: true }]);
  });
});
