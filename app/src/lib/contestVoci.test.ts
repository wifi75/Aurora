import { describe, expect, it } from 'vitest';
import { newTextContest, type ContestDeclaration, type ExchangeField, type TextContest } from '../api/contestTesto';
import { BANDE, VOCI, bandaDi, bandeAmmesse, blocchiRegolamento, leggiScore, modiAmmessi, moltiplicatoriPerBanda, pezziInline, voceAttiva, voceModificabile, VOCI_PERSONALI, type Voce } from './contestVoci';

const campo = (nome: string, tipo: string, extra: Partial<ExchangeField> = {}): ExchangeField => ({ nome, tipo, ...extra });
const dich = (p: Partial<ContestDeclaration>): ContestDeclaration => ({
  bande: [],
  modi: { ammessi: [], limite: [] },
  categorie: {},
  doppi: { per: '' },
  edizione: null,
  cabrillo: {},
  ...p,
});
const attive = (c: TextContest): Voce[] => [...VOCI, ...BANDE.map((b) => `band:${b}` as const)].filter((v) => voceAttiva(c, v));

/** ARRL 10 metri come lo da' il core: cartella, CW e SSB, solo 10 m, doppi per modo, scambio stato/provincia o numero. */
const arrl10: TextContest = {
  ...newTextContest(),
  id: 'ARRL 10 Meter Contest',
  name: 'ARRL 10 Meter Contest',
  origine: 'cartella',
  score: '; commento\nbands=10\npoints=mode:CW=4; any=2\nmult=exch/mode/if:dxcc:K,VE,XE,KH6,KL; dxcc/mode/except:K,VE,XE,KH6,KL',
  valid_cw: true,
  valid_phone: true,
  valid_digital: false,
  dupe_no_band: true,
  auto_inc: true,
  grab_number: true,
  grab_number_type: 0,
  macros: 'macros/ARRL 10 Meter Contest.mc',
  domain_file: 'dom/stati.dom',
  start: '20261212000000',
  end: '20261213235959',
  work_time: 36,
  scambio: {
    ricevuto: [campo('rst', 'rst'), campo('stato', 'testo', { elenco: 'dati/stati.dom' }), campo('numero', 'numero')],
    inviato: [campo('rst', 'rst'), campo('stato', 'testo'), campo('numero', 'numero', { progressivo: true })],
  },
  dichiarazione: dich({
    bande: ['28 MHz'],
    modi: { ammessi: ['cw', 'ssb'], limite: [{ modo: 'cw', frequenza_massima_khz: 28300 }] },
    doppi: { per: 'modo' },
    categorie: { potenza: ['QRP', 'LOW', 'HIGH'] },
    edizione: { mese: 12, durata_ore: 48, ore_massime: 36, pausa_minima_min: 30, invio_log_giorni: 7, date: { anno: 2026, inizio: '20261212000000', fine: '20261213235959', confermata: true } },
  }),
};

/** ARRL DX CW: cartella, solo CW, tutte le bande, moltiplicatori per banda. */
const arrlDx: TextContest = {
  ...newTextContest(),
  id: 'ARRL DX CW',
  name: 'ARRL International DX Competition (CW)',
  origine: 'cartella',
  score: 'bands=160,80,40,20,15,10\npoints=any=3\nmult=dxcc/band/except:K,VE',
  valid_cw: true,
  valid_phone: false,
  valid_digital: false,
  macros: 'macros/arrl-dx.mc',
  scambio: { ricevuto: [campo('rst', 'rst'), campo('potenza', 'testo')], inviato: [campo('rst', 'rst'), campo('potenza', 'testo')] },
  dichiarazione: dich({ bande: ['1.8 MHz', '3.5 MHz', '7 MHz', '14 MHz', '21 MHz', '28 MHz'], modi: { ammessi: ['cw'], limite: [] }, doppi: { per: 'banda' }, edizione: { mese: 2, durata_ore: 48, ore_massime: null, pausa_minima_min: null, invio_log_giorni: 7, date: null } }),
};

/** Un vecchio `.contest` per RTTY con le regole `[Score]`. */
const rtty: TextContest = {
  ...newTextContest(),
  id: 'CQ WW RTTY',
  name: 'CQ WW RTTY DX',
  origine: 'file',
  score: 'bands=80,40,20,15,10\npoints=same_dxcc=1; same_cont=2; other_cont=3\nmult=cq/band; dxcc/band; exch/band/if:dxcc:K,VE',
  valid_cw: false,
  valid_phone: false,
  valid_digital: true,
};

/** Un vecchio `.contest` con le sole chiavi storiche (nessuna sezione `[Score]`): moltiplicatore zona ITU, senza banda. */
const storico: TextContest = { ...newTextContest(), id: 'Vecchio ITU', name: 'Vecchio ITU', origine: 'file', valid_digital: true, points_dx: 3, multipliers: 4 };

/** UBA, fonia: file con le regole, moltiplicatori per banda sul club (scambio). */
const uba: TextContest = {
  ...newTextContest(),
  id: 'UBA SSB',
  name: 'UBA ON SSB',
  origine: 'file',
  score: 'bands=80,40,20,15,10\npoints=same_dxcc=10; same_cont=5; any=3\nmult=exch/band/if:dxcc:ON/skip:XXX; dxcc/band/except:ON',
  valid_cw: false,
  valid_phone: true,
  valid_digital: false,
};

describe('voceAttiva: ARRL 10 metri in cartella', () => {
  it('CW e fonia si, digitali no', () => {
    expect(voceAttiva(arrl10, 'valid_cw')).toBe(true);
    expect(voceAttiva(arrl10, 'valid_phone')).toBe(true);
    expect(voceAttiva(arrl10, 'valid_digital')).toBe(false);
  });
  it('una banda sola (10 m): le altre non ci sono e i moltiplicatori non sono per banda', () => {
    expect(bandeAmmesse(arrl10)).toEqual(['10']);
    expect(moltiplicatoriPerBanda(arrl10)).toBe(false);
    for (const b of BANDE) expect(voceAttiva(arrl10, `band:${b}`)).toBe(false);
    expect(voceAttiva(arrl10, 'band_mult')).toBe(false);
    // con una banda sola il doppio non dipende dalla banda; dipende dal modo
    expect(voceAttiva(arrl10, 'dupe_no_band')).toBe(false);
    expect(voceAttiva(arrl10, 'dupe_no_mode')).toBe(true);
  });
  it('lo scambio dichiarato: stato o provincia da mandare, numero progressivo, numero da leggere, elenco degli stati', () => {
    for (const v of ['mySect', 'nrs', 'auto_inc', 'grab_number', 'grab_number_type', 'domain_file', 'grab_state', 'suggest_domain'] as const) expect(voceAttiva(arrl10, v), v).toBe(true);
  });
  it('niente zone CQ/ITU, QTC, punti e moltiplicatori vecchi, contatori', () => {
    for (const v of ['suggest_cq_zone', 'suggest_itu_zone', 'qtc', 'points_dx', 'points_my_dxcc', 'multipliers', 'multipliers_option', 'valid_contacts', 'grab_time', 'log_bar_mode'] as const) expect(voceAttiva(arrl10, v), v).toBe(false);
    // i moltiplicatori sono paesi: il prefisso si suggerisce
    expect(voceAttiva(arrl10, 'suggest_dxcc_pfx')).toBe(true);
  });
  it('date, ore massime e macro dichiarate', () => {
    for (const v of ['start', 'end', 'work_time', 'macros', 'merge_macros', 'name', 'notes'] as const) expect(voceAttiva(arrl10, v), v).toBe(true);
  });
  it('in cartella si cambiano solo le voci personali: date, ore, numero, incremento, sezione e macro', () => {
    for (const v of ['start', 'end', 'work_time', 'nrs', 'auto_inc', 'mySect', 'macros'] as const) expect(voceModificabile(arrl10, v), v).toBe(true);
    expect([...VOCI_PERSONALI].sort()).toEqual(['auto_inc', 'macros', 'mySect', 'nrs', 'start', 'end', 'work_time'].sort());
  });
  it('in cartella le regole, il nome, i modi, le bande e i contatori restano in sola lettura', () => {
    for (const v of VOCI) if (!(VOCI_PERSONALI as readonly string[]).includes(v)) expect(voceModificabile(arrl10, v), v).toBe(false);
    for (const v of ['valid_cw', 'name', 'merge_macros', 'dupe_no_mode', 'band:10', 'points_dx', 'multipliers', 'qtc'] as const) expect(voceModificabile(arrl10, v), v).toBe(false);
  });
});

describe('voceAttiva: ARRL DX', () => {
  it('solo CW, tutte le bande con i moltiplicatori per banda', () => {
    expect(voceAttiva(arrlDx, 'valid_cw')).toBe(true);
    expect(voceAttiva(arrlDx, 'valid_phone')).toBe(false);
    expect(voceAttiva(arrlDx, 'valid_digital')).toBe(false);
    expect(bandeAmmesse(arrlDx)).toEqual(['160', '80', '40', '20', '15', '10']);
    for (const b of BANDE) expect(voceAttiva(arrlDx, `band:${b}`)).toBe(true);
    expect(voceAttiva(arrlDx, 'band_mult')).toBe(true);
    expect(voceAttiva(arrlDx, 'dupe_no_band')).toBe(true);
  });
  it('lo scambio è rapporto e potenza: nessun numero progressivo, nessun elenco di valori', () => {
    // niente moltiplicatore sullo scambio e nessun campo di testo con un elenco: ma la potenza e' un campo di testo da mandare
    expect(voceAttiva(arrlDx, 'nrs')).toBe(false);
    expect(voceAttiva(arrlDx, 'auto_inc')).toBe(false);
    expect(voceAttiva(arrlDx, 'grab_number')).toBe(false);
    expect(voceAttiva(arrlDx, 'domain_file')).toBe(false);
    expect(voceAttiva(arrlDx, 'work_time')).toBe(false);
  });
});

describe('voceAttiva: vecchi .contest', () => {
  it('RTTY: CW e fonia spenti, digitali accesi', () => {
    expect(voceAttiva(rtty, 'valid_cw')).toBe(false);
    expect(voceAttiva(rtty, 'valid_phone')).toBe(false);
    expect(voceAttiva(rtty, 'valid_digital')).toBe(true);
  });
  it('RTTY: bande di bands=, moltiplicatori per banda, zone CQ si e ITU no, sezione propria, punti vecchi spenti perche\' ci sono le regole', () => {
    expect(bandeAmmesse(rtty)).toEqual(['80', '40', '20', '15', '10']);
    expect(voceAttiva(rtty, 'band:160')).toBe(false);
    expect(voceAttiva(rtty, 'band:80')).toBe(true);
    expect(voceAttiva(rtty, 'band_mult')).toBe(true);
    expect(voceAttiva(rtty, 'suggest_cq_zone')).toBe(true);
    expect(voceAttiva(rtty, 'suggest_itu_zone')).toBe(false);
    expect(voceAttiva(rtty, 'suggest_dxcc_pfx')).toBe(true);
    expect(voceAttiva(rtty, 'mySect')).toBe(true);
    expect(voceAttiva(rtty, 'points_dx')).toBe(false);
    expect(voceAttiva(rtty, 'multipliers')).toBe(false);
    expect(voceAttiva(rtty, 'qtc')).toBe(false);
  });
  it('RTTY: un vecchio file e\' tutto modificabile', () => {
    for (const v of VOCI) expect(voceModificabile(rtty, v)).toBe(true);
  });
  it('senza [Score]: valgono le chiavi storiche (moltiplicatore ITU, nessuna banda per banda)', () => {
    expect(voceAttiva(storico, 'points_dx')).toBe(true);
    expect(voceAttiva(storico, 'multipliers')).toBe(true);
    expect(voceAttiva(storico, 'suggest_itu_zone')).toBe(true);
    expect(voceAttiva(storico, 'suggest_cq_zone')).toBe(false);
    expect(voceAttiva(storico, 'suggest_dxcc_pfx')).toBe(false);
    expect(voceAttiva(storico, 'mySect')).toBe(false);
    // la banda per banda segue il tipo di moltiplicatore scelto
    expect(voceAttiva(storico, 'band:20')).toBe(true);
    expect(voceAttiva({ ...storico, multipliers: 0 }, 'band_mult')).toBe(false);
  });
  it('UBA: fonia, bande per banda, sezione propria (club); CW e digitali spenti', () => {
    expect(voceAttiva(uba, 'valid_phone')).toBe(true);
    expect(voceAttiva(uba, 'valid_cw')).toBe(false);
    expect(voceAttiva(uba, 'valid_digital')).toBe(false);
    expect(voceAttiva(uba, 'mySect')).toBe(true);
    expect(voceAttiva(uba, 'band:160')).toBe(false);
    expect(voceAttiva(uba, 'band:40')).toBe(true);
    expect(voceAttiva(uba, 'suggest_cq_zone')).toBe(false);
  });
  it('il QTC si vede solo se il contest lo prevede', () => {
    expect(voceAttiva({ ...rtty, qtc: 2 }, 'qtc')).toBe(true);
    expect(voceAttiva({ ...rtty, qtc: 0 }, 'qtc')).toBe(false);
  });
  it('senza nessun modo acceso tutti i modi valgono; i modi nominati dalle regole contano', () => {
    const tutti = { ...rtty, valid_digital: false, score: 'points=any=1' };
    expect([...modiAmmessi(tutti)].sort()).toEqual(['cw', 'digital', 'phone']);
    const cwRtty = { ...rtty, valid_digital: false, score: 'points=mode:CW=2; mode:RTTY=1' };
    expect([...modiAmmessi(cwRtty)].sort()).toEqual(['cw', 'digital']);
  });
});

describe('voceAttiva: contest nuovo', () => {
  it('tutto attivo tranne la propria sezione (non c\'e\' ancora un id)', () => {
    const nuovo = newTextContest();
    expect(attive(nuovo)).toHaveLength(VOCI.length - 1 + BANDE.length);
    expect(voceAttiva(nuovo, 'mySect')).toBe(false);
  });
});

describe('regole [Score] e utilita\'', () => {
  it('le righe commento non contano e i pezzi di mult si leggono con la banda', () => {
    const r = leggiScore('; bands=40\nbands=20,15\nmult=dxcc/band; area:K,VE,JA/mode; exch/if:dxcc:SV,SV/A');
    expect(r.bande).toEqual(['20', '15']);
    expect(r.mult).toEqual([
      { tipo: 'dxcc', perBanda: true },
      { tipo: 'area', perBanda: false },
      { tipo: 'exch', perBanda: false },
    ]);
  });
  it('le bande scritte come frequenza o come lunghezza d\'onda', () => {
    expect(bandaDi('28 MHz')).toBe('10');
    expect(bandaDi('3.5 MHz')).toBe('80');
    expect(bandaDi('1.8 MHz')).toBe('160');
    expect(bandaDi('40m')).toBe('40');
    expect(bandaDi('boh')).toBeNull();
  });
});

describe('regolamento in markdown semplice', () => {
  it('titoli, paragrafi di piu\' righe ed elenchi; nessun HTML interpretato', () => {
    const b = blocchiRegolamento('# Regole\n\nPrimo rigo\nsecondo rigo\n\n- uno\n- due\n\n<b>testo</b>\n1. numerato');
    expect(b).toEqual([
      { tipo: 'titolo', livello: 1, testo: 'Regole' },
      { tipo: 'paragrafo', testo: 'Primo rigo secondo rigo' },
      { tipo: 'elenco', voci: ['uno', 'due'] },
      { tipo: 'paragrafo', testo: '<b>testo</b>' },
      { tipo: 'elenco', voci: ['numerato'] },
    ]);
    expect(blocchiRegolamento(null)).toEqual([]);
  });
});

describe('pezziInline', () => {
  it('rende grassetto, codice e link web, e lascia il resto com e', () => {
    const p = pezziInline('**Fonte:** regole <https://www.arrl.org/x.pdf> e `a=usa` e [sito](https://arrl.org) fine');
    expect(p.map((x) => x.tipo)).toEqual(['grassetto', 'testo', 'link', 'testo', 'codice', 'testo', 'link', 'testo']);
    expect(p[0]).toEqual({ tipo: 'grassetto', testo: 'Fonte:' });
    expect(p[2]).toEqual({ tipo: 'link', testo: 'https://www.arrl.org/x.pdf', url: 'https://www.arrl.org/x.pdf' });
    expect(p[4]).toEqual({ tipo: 'codice', testo: 'a=usa' });
    expect(p[6]).toEqual({ tipo: 'link', testo: 'sito', url: 'https://arrl.org' });
  });
  it('non fa link di schemi diversi da http e https', () => {
    const p = pezziInline('[x](javascript:alert(1)) <file:///c:/x> [y](ftp://a/b)');
    expect(p.every((x) => x.tipo !== 'link')).toBe(true);
  });
  it('una riga senza segni e un solo pezzo di testo', () => {
    expect(pezziInline('solo testo')).toEqual([{ tipo: 'testo', testo: 'solo testo' }]);
  });
});
