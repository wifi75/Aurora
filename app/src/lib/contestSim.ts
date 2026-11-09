/**
 * Simulatore di contest per l'allenamento (solo nella pagina, mai nel motore): genera le decodifiche finte di un pile-up che risponde al
 * nostro CQ con lo scambio del contest scelto, con errori, doppi e chi non riceve l'RR73. Le nostre «trasmissioni» sono solo righe a video:
 * nessun PTT, nessun comando alla radio, niente nel log vero ne' nel log del contest. Il punteggio resta qui dentro.
 */

export type Livello = 'leggero' | 'medio' | 'fitto';

/** Una riga della tabella: decodifica finta o nostra trasmissione finta. */
export interface SimRiga {
  id: number;
  time_ms: number;
  snr: number;
  freq_hz: number;
  text: string;
  call: string;
  /** `tx` = la nostra trasmissione (finta). */
  tx?: boolean;
  doppio?: boolean;
  /** Lo scambio ricevuto non e' valido per il contest. */
  sbagliato?: boolean;
  /** Primo QSO con quel moltiplicatore. */
  nuovo?: string;
}

export interface SimQso {
  call: string;
  scambio: string;
  time_ms: number;
  stato: 'attesa' | 'valido' | 'incerto';
  doppio: boolean;
  errore: boolean;
  molt: string;
}

export interface SimPunteggio {
  qso: number;
  doppi: number;
  errori: number;
  incerti: number;
  nilEvitati: number;
  molt: number;
  punteggio: number;
  rate: number;
}

interface Chiamante {
  call: string;
  scambio: string;
  sbagliato: boolean;
  snr: number;
  freq: number;
  pazienza: number;
  /** Fase con noi: 0 chiama, 1 ha ricevuto il nostro scambio, 2 ha mandato R+scambio, 3 aspetta RR73 ancora (non l'ha ricevuto). */
  fase: number;
  nonRiceveRr73: boolean;
  nonCopia: boolean;
  /** Gia' nel log con questo QSO (un secondo RR73 non fa un altro QSO). */
  loggato: boolean;
}

const PERIODO = 15_000;
const PREFISSI: [string, string[]][] = [
  ['K', ['FN42', 'EM73', 'DM79', 'CN87', 'EN52', 'FM18']],
  ['W', ['FN31', 'EM12', 'DM13', 'EN61', 'FM29', 'EL98']],
  ['N', ['EM48', 'FN20', 'DN70', 'CM87']],
  ['VE', ['FN25', 'FN03', 'EN82', 'DO21']],
  ['DL', ['JO62', 'JN58', 'JO31', 'JO40']],
  ['F', ['JN18', 'IN94', 'JN03', 'JN23']],
  ['G', ['IO91', 'IO83', 'IO92']],
  ['EA', ['IN80', 'IM87', 'JN11']],
  ['I', ['JN45', 'JN61', 'JM78', 'JN54']],
  ['OK', ['JO70', 'JN79']],
  ['SP', ['JO92', 'KO02']],
  ['JA', ['PM95', 'PM74', 'QN02']],
  ['PY', ['GG66', 'GG87']],
  ['VK', ['QF56', 'QF22']],
  ['LU', ['GF05']],
  ['ZS', ['KG33']],
  ['UA', ['KO85', 'LO87']],
  ['YO', ['KN34']],
  ['OH', ['KP20']],
  ['SM', ['JO89']],
];
const SEZIONI = ['EMA', 'WMA', 'CT', 'NNY', 'ENY', 'NLI', 'OH', 'MI', 'IL', 'WI', 'STX', 'NTX', 'SCV', 'LAX', 'ORG', 'GA', 'NFL', 'ONE', 'GTA', 'DX'];
const STATI = ['MA', 'CT', 'NY', 'OH', 'MI', 'IL', 'TX', 'CA', 'FL', 'GA', 'WA', 'OR', 'CO', 'AZ', 'VA', 'NC', 'ON', 'QC', 'BC', 'DX'];
const PROVINCE = ['MI', 'TO', 'RM', 'NA', 'BO', 'FI', 'GE', 'VE', 'PA', 'BA', 'TS', 'PD'];
const LETTERE = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';

const caso = (n: number) => Math.floor(Math.random() * n);
const uno = <T,>(a: readonly T[]): T => a[caso(a.length)] as T;

/** Pile-up: quante stazioni nuove chiamano a ogni periodo e quanto spesso sbagliano. */
const PROFILO: Record<Livello, { nuovi: [number, number]; errore: number; doppio: number }> = {
  leggero: { nuovi: [0, 2], errore: 0.03, doppio: 0.05 },
  medio: { nuovi: [1, 3], errore: 0.05, doppio: 0.08 },
  fitto: { nuovi: [3, 6], errore: 0.08, doppio: 0.1 },
};

const rapporto = (snr: number) => `${snr < 0 ? '-' : '+'}${String(Math.abs(snr)).padStart(2, '0')}`;

export class Simulatore {
  readonly fine_ms: number;
  readonly inizio_ms: number;
  private campi: string[];
  private mio: string;
  private mioScambio: string;
  private parolaCq: string;
  private livello: Livello;
  private chiamanti: Chiamante[] = [];
  private lavorati = new Map<string, SimQso>();
  private molt = new Set<string>();
  private prossimoId = 1;
  private seriale = 1;
  /** La nostra prossima trasmissione finta: scambio a un chiamante o RR73. */
  private tx: { tipo: 'scambio' | 'rr73'; call: string } | null = null;
  private ultimoPeriodo = 0;
  righe: SimRiga[] = [];
  log: SimQso[] = [];
  /** Il nominativo con cui siamo in QSO (finto). */
  dx: string | null = null;
  nilEvitati = 0;

  constructor(o: { campi: string[]; mio: string; mioGrid: string; parolaCq: string; livello: Livello; minuti: number; ora?: number }) {
    const ora = o.ora ?? Date.now();
    this.inizio_ms = ora;
    this.fine_ms = ora + o.minuti * 60_000;
    this.campi = o.campi.length ? o.campi : ['grid4'];
    this.mio = o.mio || 'IU0XXX';
    this.parolaCq = o.parolaCq;
    this.livello = o.livello;
    this.mioScambio = this.scambioDi(o.mioGrid.slice(0, 4) || 'JN45', true);
    this.ultimoPeriodo = Math.floor(ora / PERIODO);
  }

  get finito() {
    return Date.now() >= this.fine_ms;
  }

  /** Lo scambio di una stazione per i campi del contest (il seriale e' quello che manderebbe lei). */
  private scambioDi(grid: string, mio = false): string {
    return this.campi
      .map((c) => {
        switch (c) {
          case 'grid4':
          case 'grid6':
            return grid;
          case 'rst':
          case 'snr':
            return rapporto(-caso(20));
          case 'seriale':
            return String(mio ? this.seriale : 1 + caso(400)).padStart(3, '0');
          case 'classe':
            return `${1 + caso(3)}${uno(['A', 'B', 'D', 'E'])}`;
          case 'sezione':
            return uno(SEZIONI);
          case 'stato':
            return uno(STATI);
          case 'provincia':
            return uno(PROVINCE);
          default:
            return grid;
        }
      })
      .join(' ');
  }

  /** Il moltiplicatore che porta uno scambio (semplificato: il primo campo, il campo del locatore per i locatori). */
  private moltDi(scambio: string): string {
    const v = scambio.split(' ')[0] ?? '';
    return /^[A-R]{2}\d{2}$/i.test(v) ? v.slice(0, 2) : v;
  }

  private nuovoChiamante(): Chiamante {
    const p = PROFILO[this.livello];
    // ogni tanto richiama chi e' gia' nel log: un doppio
    if (this.lavorati.size && Math.random() < p.doppio) {
      const q = uno([...this.lavorati.values()]);
      return this.chiamante(q.call, q.scambio, false);
    }
    const [pre, griglie] = uno(PREFISSI);
    let call = '';
    for (let i = 0; i < 20; i++) {
      const suff = Array.from({ length: 1 + caso(3) }, () => LETTERE[caso(26)]).join('');
      call = `${pre}${caso(10)}${suff}`;
      if (call !== this.mio && !this.chiamanti.some((c) => c.call === call)) break;
    }
    const grid = uno(griglie);
    let scambio = this.scambioDi(grid);
    const sbagliato = Math.random() < p.errore;
    // l'errore tipico: il rapporto al posto dello scambio del contest
    if (sbagliato) scambio = rapporto(-caso(20));
    return this.chiamante(call, scambio, sbagliato);
  }

  private chiamante(call: string, scambio: string, sbagliato: boolean): Chiamante {
    return { call, scambio, sbagliato, snr: -22 + caso(30), freq: 300 + caso(2400), pazienza: 2 + caso(5), fase: 0, nonRiceveRr73: Math.random() < 0.12, nonCopia: Math.random() < 0.1, loggato: false };
  }

  private riga(text: string, call: string, c?: Chiamante, extra?: Partial<SimRiga>): void {
    this.righe.push({ id: this.prossimoId++, time_ms: this.ultimoPeriodo * PERIODO, snr: c ? c.snr : 0, freq_hz: c ? c.freq : 1500, text, call, ...extra });
  }

  /** Chiamata dalla pagina ogni secondo (solo con la linguetta aperta): fa avanzare un periodo quando e' il momento. Torna vero se e' cambiato. */
  avanza(ora = Date.now()): boolean {
    const periodo = Math.floor(ora / PERIODO);
    if (periodo <= this.ultimoPeriodo || this.finito) return false;
    this.ultimoPeriodo = periodo;
    this.periodo();
    return true;
  }

  private periodo(): void {
    // le righe di prima restano visibili per due periodi, come nella vista delle decodifiche vere
    const limite = (this.ultimoPeriodo - 2) * PERIODO;
    this.righe = this.righe.filter((r) => r.time_ms >= limite);
    // 1) la nostra trasmissione (finta) del periodo appena chiuso
    const tx = this.tx;
    this.tx = null;
    if (tx) {
      const c = this.chiamanti.find((x) => x.call === tx.call);
      if (tx.tipo === 'scambio') {
        this.riga(`${tx.call} ${this.mio} ${this.mioScambio}`, tx.call, undefined, { tx: true });
        if (c) c.fase = c.nonCopia ? 0 : 1;
        if (c) c.nonCopia = false;
      } else {
        this.riga(`${tx.call} ${this.mio} RR73`, tx.call, undefined, { tx: true });
        this.registra(tx.call);
        if (c && c.fase === 3) {
          // lo ha ricevuto alla seconda: QSO salvo
          this.nilEvitati++;
          c.nonRiceveRr73 = false;
        }
        if (c) c.fase = c.nonRiceveRr73 ? 3 : 4;
      }
    } else if (!this.dx) {
      this.riga(`CQ ${this.parolaCq ? this.parolaCq + ' ' : ''}${this.mio} ${this.mioScambio.split(' ')[0]}`, this.mio, undefined, { tx: true });
    }
    // 2) le risposte dei chiamanti
    const p = PROFILO[this.livello];
    const via: string[] = [];
    for (const c of this.chiamanti) {
      c.snr = Math.max(-24, Math.min(15, c.snr + caso(5) - 2));
      if (c.fase === 0) {
        if (--c.pazienza < 0) {
          via.push(c.call);
          continue;
        }
        this.riga(`${this.mio} ${c.call} ${c.scambio}`, c.call, c, { doppio: this.lavorati.has(c.call), sbagliato: c.sbagliato });
      } else if (c.fase === 1 || c.fase === 2) {
        this.riga(`${this.mio} ${c.call} R ${c.scambio}`, c.call, c, { sbagliato: c.sbagliato });
        c.fase = 2;
      } else if (c.fase === 3) {
        // non ha ricevuto l'RR73: ripete; se non lo rimandiamo il QSO resta incerto
        this.riga(`${this.mio} ${c.call} R ${c.scambio}`, c.call, c, { sbagliato: c.sbagliato });
        const q = this.lavorati.get(c.call);
        if (q && q.stato === 'attesa') q.stato = 'incerto';
        if (--c.pazienza < -3) via.push(c.call);
      } else if (c.fase === 4) {
        this.riga(`${this.mio} ${c.call} 73`, c.call, c);
        const q = this.lavorati.get(c.call);
        if (q && q.stato !== 'valido') q.stato = 'valido';
        via.push(c.call);
      }
    }
    this.chiamanti = this.chiamanti.filter((c) => !via.includes(c.call));
    if (this.dx && !this.chiamanti.some((c) => c.call === this.dx)) this.dx = null;
    // 3) nuovi chiamanti (finche' l'allenamento non sta per finire)
    if (this.fine_ms - this.ultimoPeriodo * PERIODO > PERIODO) {
      const [a, b] = p.nuovi;
      const n = a + caso(b - a + 1);
      for (let i = 0; i < n && this.chiamanti.length < 14; i++) {
        const c = this.nuovoChiamante();
        if (this.chiamanti.some((x) => x.call === c.call)) continue;
        this.chiamanti.push(c);
        this.riga(`${this.mio} ${c.call} ${c.scambio}`, c.call, c, { doppio: this.lavorati.has(c.call), sbagliato: c.sbagliato, nuovo: this.molt.has(this.moltDi(c.scambio)) || c.sbagliato ? undefined : this.moltDi(c.scambio) });
      }
    }
  }

  private registra(call: string): void {
    const c = this.chiamanti.find((x) => x.call === call);
    if (c?.loggato) return;
    if (c) c.loggato = true;
    const prima = this.lavorati.get(call);
    if (prima) {
      // QSO ripetuto: doppio nel log, zero punti
      this.log.push({ ...prima, time_ms: this.ultimoPeriodo * PERIODO, doppio: true, stato: 'valido' });
      return;
    }
    const scambio = c?.scambio ?? '';
    const errore = !!c?.sbagliato;
    const molt = errore ? '' : this.moltDi(scambio);
    if (molt) this.molt.add(molt);
    const q: SimQso = { call, scambio, time_ms: this.ultimoPeriodo * PERIODO, stato: 'attesa', doppio: false, errore, molt };
    this.lavorati.set(call, q);
    this.log.push(q);
    if (this.campi.includes('seriale')) {
      this.seriale++;
      this.mioScambio = this.mioScambio.replace(/\b\d{3,}\b/, String(this.seriale).padStart(3, '0'));
    }
  }

  /** Doppio clic su una chiamata: il prossimo periodo mandiamo lo scambio (o RR73 se ci ha gia' mandato R+scambio). */
  rispondi(call: string): void {
    if (this.finito) return;
    const c = this.chiamanti.find((x) => x.call === call);
    if (!c) return;
    this.dx = call;
    this.tx = c.fase >= 2 ? { tipo: 'rr73', call } : { tipo: 'scambio', call };
  }

  /** Il pulsante RR73: chiude con la stazione in QSO (o la rimanda a chi l'ha persa). */
  rr73(): void {
    if (this.dx && !this.finito) this.tx = { tipo: 'rr73', call: this.dx };
  }

  /** Lo scambio che manda la stazione in QSO, la fase e cosa partira' al prossimo periodo (per il riquadro del QSO). */
  inQso(): { call: string; fase: number; prossimo: string } | null {
    if (!this.dx) return null;
    const c = this.chiamanti.find((x) => x.call === this.dx);
    return { call: this.dx, fase: c?.fase ?? 0, prossimo: this.tx ? this.tx.tipo : '' };
  }

  punteggio(ora = Date.now()): SimPunteggio {
    const buoni = this.log.filter((q) => !q.doppio && !q.errore);
    const ore = Math.max(1 / 60, (Math.min(ora, this.fine_ms) - this.inizio_ms) / 3_600_000);
    const molt = new Set(buoni.map((q) => q.molt).filter(Boolean)).size;
    return {
      qso: buoni.length,
      doppi: this.log.filter((q) => q.doppio).length,
      errori: this.log.filter((q) => q.errore).length,
      incerti: buoni.filter((q) => q.stato === 'incerto').length,
      nilEvitati: this.nilEvitati,
      molt,
      // punteggio semplificato: 1 punto per QSO buono e non incerto, per i moltiplicatori
      punteggio: buoni.filter((q) => q.stato !== 'incerto').length * Math.max(1, molt),
      rate: Math.round(buoni.length / ore),
    };
  }
}
