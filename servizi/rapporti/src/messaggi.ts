/**
 * Messaggi del proprietario a tutte le installazioni (auguri, avvisi, «aggiorna subito»): li scrive dalla pagina /admin/messaggi con il
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

/** La pagina per scrivere i messaggi: il segreto si scrive nella pagina e resta solo in questa scheda del browser. */
export const PAGINA = `<!doctype html>
<html lang="it"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="robots" content="noindex"><title>Aurora - messaggi</title>
<style>
:root{--bg:#0b1220;--card:#101b2e;--line:#1e2c44;--text:#e6edf3;--muted:#8fa0b5;--acc:#2cf5a4;--vio:#8e7bff}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:15px/1.5 system-ui,sans-serif}
main{max-width:860px;margin:0 auto;padding:24px 16px}h1{margin:0 0 4px;font-size:22px;background:linear-gradient(90deg,#2cf5a4,#27a0f5,#8e7bff);-webkit-background-clip:text;background-clip:text;color:transparent}
.card{background:var(--card);border:1px solid var(--line);border-radius:12px;padding:16px;margin:16px 0}
label{display:block;font-size:12px;color:var(--muted);margin:10px 0 4px;text-transform:uppercase;letter-spacing:.06em}
input,textarea,select{width:100%;padding:8px 10px;border-radius:8px;border:1px solid var(--line);background:#0b1626;color:var(--text);font:inherit}
textarea{min-height:70px;resize:vertical}.row{display:grid;grid-template-columns:1fr 1fr;gap:12px}
button{padding:9px 16px;border-radius:8px;border:1px solid var(--acc);background:var(--acc);color:#04342c;font-weight:700;cursor:pointer}
button.ghost{background:transparent;color:var(--text);border-color:var(--line)}.muted{color:var(--muted);font-size:13px}
.msg{border-top:1px solid var(--line);padding:10px 0;display:flex;gap:12px;justify-content:space-between;align-items:flex-start}
.on{color:var(--acc)}.off{color:var(--muted)}#esito{margin-left:10px}details{margin-top:8px}
</style></head><body><main>
<h1>Messaggi di Aurora</h1><p class="muted">Arrivano a tutte le installazioni come cartolina con il suono, una volta sola. Sempre accesi.</p>
<div class="card"><label>Segreto dell'amministratore</label><input id="tok" type="password" autocomplete="off" placeholder="ADMIN_TOKEN">
<p class="muted">Resta solo in questa scheda del browser.</p></div>
<div class="card"><b>Nuovo messaggio</b>
<label>Titolo (italiano)</label><input id="t_it" maxlength="120">
<label>Testo (italiano)</label><textarea id="x_it" maxlength="1200"></textarea>
<details><summary class="muted">Altre lingue (facoltative: chi non ha la sua lingua vede l'inglese o l'italiano)</summary>
<label>Titolo (English)</label><input id="t_en"><label>Testo (English)</label><textarea id="x_en"></textarea>
<label>Titolo (Deutsch)</label><input id="t_de"><label>Testo (Deutsch)</label><textarea id="x_de"></textarea>
<label>Titolo (Français)</label><input id="t_fr"><label>Testo (Français)</label><textarea id="x_fr"></textarea>
<label>Titolo (Español)</label><input id="t_es"><label>Testo (Español)</label><textarea id="x_es"></textarea>
<label>Titolo (Português)</label><input id="t_pt"><label>Testo (Português)</label><textarea id="x_pt"></textarea>
</details>
<div class="row"><div><label>Dal (ora del tuo PC)</label><input id="dal" type="datetime-local"></div><div><label>Al</label><input id="al" type="datetime-local"></div></div>
<div class="row"><div><label>Solo alle versioni piu' vecchie di (facoltativo)</label><input id="prima" placeholder="es. 0.11.167"></div>
<div><label>Aspetto</label><select id="tema"><option value="normale">Normale</option><option value="festa">Festa (auguri)</option></select></div></div>
<label>Collegamento (facoltativo, https://...)</label><input id="link" placeholder="https://aurora.iu3wjq.com/">
<p><button id="invia">Pubblica il messaggio</button><span id="esito" class="muted"></span></p></div>
<div class="card"><b>Messaggi</b> <button class="ghost" id="ricarica" style="float:right;padding:4px 10px">Aggiorna</button><div id="lista" class="muted">Scrivi il segreto e premi Aggiorna.</div></div>
</main><script>
const $=(id)=>document.getElementById(id);const L=['it','en','de','fr','es','pt'];
try{$('tok').value=sessionStorage.getItem('aurora-admin')||''}catch{}
$('tok').addEventListener('input',()=>{try{sessionStorage.setItem('aurora-admin',$('tok').value)}catch{}});
const now=new Date();const f=(d)=>new Date(d.getTime()-d.getTimezoneOffset()*6e4).toISOString().slice(0,16);
$('dal').value=f(now);$('al').value=f(new Date(now.getTime()+3*864e5));
const h=()=>({Authorization:'Bearer '+$('tok').value,'Content-Type':'application/json'});
const esc=(s)=>String(s).replace(/[&<>"]/g,(c)=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
async function carica(){const r=await fetch('/v1/messaggi/tutti',{headers:h()});if(!r.ok){$('lista').textContent=r.status===401?'Segreto sbagliato.':'Errore '+r.status;return}
const {messaggi}=await r.json();const ora=new Date().toISOString();
$('lista').innerHTML=messaggi.length?messaggi.map((m)=>{const on=m.dal<=ora&&m.al>=ora;return '<div class="msg"><div><b>'+esc(m.titolo.it||m.titolo.en||'')+'</b> <span class="'+(on?'on':'off')+'">'+(on?'in onda':(m.al<ora?'finito':'in attesa'))+'</span><br><span class="muted">'+new Date(m.dal).toLocaleString()+' - '+new Date(m.al).toLocaleString()+(m.prima_di?' - solo prima della '+esc(m.prima_di):'')+(m.tema==='festa'?' - festa':'')+'</span><br>'+esc(m.testo.it||m.testo.en||'')+'</div><button class="ghost" data-id="'+m.id+'">Togli</button></div>'}).join(''):'Nessun messaggio.';
document.querySelectorAll('[data-id]').forEach((b)=>b.onclick=async()=>{if(!confirm('Togliere questo messaggio?'))return;await fetch('/v1/messaggi/'+b.dataset.id,{method:'DELETE',headers:h()});carica()})}
$('ricarica').onclick=carica;
$('invia').onclick=async()=>{const titolo={},testo={};for(const l of L){titolo[l]=$('t_'+l).value;testo[l]=$('x_'+l).value}
const corpo={titolo,testo,dal:new Date($('dal').value).toISOString(),al:new Date($('al').value).toISOString(),prima_di:$('prima').value,link:$('link').value,tema:$('tema').value};
const r=await fetch('/v1/messaggi',{method:'POST',headers:h(),body:JSON.stringify(corpo)});const j=await r.json().catch(()=>({}));
$('esito').textContent=r.ok?'Pubblicato.':(r.status===401?'Segreto sbagliato.':(j.error||'Errore '+r.status));if(r.ok){for(const l of L){$('t_'+l).value='';$('x_'+l).value=''}carica()}};
if($('tok').value)carica();
</script></body></html>`;
