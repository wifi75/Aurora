/**
 * Messaggi del proprietario a tutte le installazioni (auguri, avvisi, «aggiorna subito»): li scrive solo da Aurora sul suo PC, con il
 * segreto dell'amministratore; Aurora (e l'agente nel tray) li legge ogni ora da GET /v1/messaggi e li mostra una volta sola.
 * Nessun dato di chi li legge: la richiesta porta solo la versione del programma, per i messaggi rivolti alle versioni vecchie.
 */

const LINGUE = ['it', 'en', 'de', 'fr', 'es', 'pt'] as const;
type Lingua = (typeof LINGUE)[number];

export interface Messaggio {
  id: string;
  titolo: Partial<Record<Lingua, string>>;
  testo: Partial<Record<Lingua, string>>;
  /** Da quando a quando si mostra (ISO, UTC). */
  dal: string;
  al: string;
  /** Solo alle versioni piu' vecchie di questa (vuoto: a tutti). */
  prima_di: string;
  link: string;
  /** Aspetto della cartolina: normale o festa (auguri). */
  tema: 'normale' | 'festa';
  creato: string;
}

function versione(v: string): number[] {
  return v.split('.').map((x) => Number.parseInt(x, 10) || 0);
}

/** `a` e' piu' vecchia di `b`. */
function piuVecchia(a: string, b: string): boolean {
  const x = versione(a);
  const y = versione(b);
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d !== 0) return d < 0;
  }
  return false;
}

function testi(v: unknown, max: number): Partial<Record<Lingua, string>> {
  const out: Partial<Record<Lingua, string>> = {};
  if (typeof v !== 'object' || v === null) return out;
  for (const l of LINGUE) {
    const t = (v as Record<string, unknown>)[l];
    if (typeof t === 'string' && t.trim() !== '') out[l] = t.replace(/[\u0000-\u0008\u000b-\u001f\u007f]/g, ' ').trim().slice(0, max);
  }
  return out;
}

function data(v: unknown): string | null {
  if (typeof v !== 'string' || v === '') return null;
  const d = new Date(v);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

export function daRiga(r: Record<string, unknown>): Messaggio {
  return {
    id: String(r.id),
    titolo: JSON.parse(String(r.titolo || '{}')) as Messaggio['titolo'],
    testo: JSON.parse(String(r.testo || '{}')) as Messaggio['testo'],
    dal: String(r.dal),
    al: String(r.al),
    prima_di: String(r.prima_di ?? ''),
    link: String(r.link ?? ''),
    tema: r.tema === 'festa' ? 'festa' : 'normale',
    creato: String(r.creato),
  };
}

/** I messaggi da mostrare adesso a una certa versione (pubblico). */
export async function attivi(db: D1Database, ver: string): Promise<Messaggio[]> {
  const ora = new Date().toISOString();
  const { results } = await db.prepare('SELECT * FROM messaggi WHERE dal <= ?1 AND al >= ?1 ORDER BY dal DESC LIMIT 20').bind(ora).all();
  return results.map(daRiga).filter((m) => m.prima_di === '' || ver === '' || piuVecchia(ver, m.prima_di));
}

/** Tutti i messaggi (amministratore). */
export async function tutti(db: D1Database): Promise<Messaggio[]> {
  const { results } = await db.prepare('SELECT * FROM messaggi ORDER BY dal DESC LIMIT 200').all();
  return results.map(daRiga);
}

/** Crea un messaggio dal corpo JSON (amministratore). Restituisce l'errore in chiaro se manca qualcosa. */
export async function crea(db: D1Database, corpo: unknown): Promise<Messaggio | string> {
  if (typeof corpo !== 'object' || corpo === null) return 'corpo non valido';
  const c = corpo as Record<string, unknown>;
  const titolo = testi(c.titolo, 120);
  const testo = testi(c.testo, 1200);
  if (!titolo.it && !titolo.en) return 'serve almeno il titolo in italiano o in inglese';
  if (!testo.it && !testo.en) return 'serve almeno il testo in italiano o in inglese';
  const dal = data(c.dal) ?? new Date().toISOString();
  const al = data(c.al);
  if (!al || al <= dal) return 'la data di fine deve venire dopo quella di inizio';
  const prima = typeof c.prima_di === 'string' && /^\d+(\.\d+){0,3}$/.test(c.prima_di.trim()) ? c.prima_di.trim() : '';
  const link = typeof c.link === 'string' && /^https:\/\/[^\s"'<>]{3,300}$/.test(c.link.trim()) ? c.link.trim() : '';
  const tema = c.tema === 'festa' ? 'festa' : 'normale';
  const id = crypto.randomUUID().slice(0, 8);
  const creato = new Date().toISOString();
  await db
    .prepare('INSERT INTO messaggi (id, titolo, testo, dal, al, prima_di, link, tema, creato) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)')
    .bind(id, JSON.stringify(titolo), JSON.stringify(testo), dal, al, prima, link, tema, creato)
    .run();
  return { id, titolo, testo, dal, al, prima_di: prima, link, tema, creato };
}

export async function togli(db: D1Database, id: string): Promise<void> {
  await db.prepare('DELETE FROM messaggi WHERE id = ?1').bind(id).run();
}
