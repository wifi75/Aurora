import { useEffect, useRef, useState } from 'react';
import { addLogQso, getCallsign, type CallsignInfo } from '../../api/client';
import { useT } from '../../i18n';
import { bandOf } from '../../lib/format';
import { replyErrorMessage } from '../../lib/replyError';
import { onSstv } from '../../lib/sstvBus';
import { toast } from '../../lib/toast';

const EMPTY = { call: '', sent: '59', rcvd: '59', name: '', qth: '', grid: '', note: '' };
type Form = typeof EMPTY;
/** I campi che la ricerca del nominativo puo' riempire. */
type Looked = Pick<Form, 'name' | 'qth' | 'grid'>;

/** Un nominativo da cercare: lettere, cifre e «/», almeno una cifra e una lettera. */
export function lookupWorthy(call: string): boolean {
  const c = call.trim().toUpperCase();
  return /^[A-Z0-9/]{3,15}$/.test(c) && /\d/.test(c) && /[A-Z]/.test(c);
}

/** Dalla risposta della ricerca: nome (quello con cui la stazione si fa chiamare, se c'e'), QTH (citta'), locatore. */
export function lookedUp(info: CallsignInfo): Looked {
  const p = info.person;
  return { name: (p?.fname || p?.name || '').trim(), qth: (p?.city ?? '').trim(), grid: (p?.grid ?? '').trim() };
}

/** Attesa dopo l'ultimo tasto prima di cercare (non si cerca a ogni lettera). */
const LOOKUP_DELAY_MS = 600;

/**
 * La scheda per mettere a log il QSO fatto in SSTV (come il riquadro Log dell'MMSSTV): nominativo, rapporti, nome, QTH, locatore, note.
 * Il nominativo arriva anche dall'FSK ID dopo l'immagine; appena c'e' un nominativo si cercano i dati della stazione con la ricerca gia'
 * configurata in Aurora (QRZ.com o HamQTH, `/callsign/{call}`) e si riempiono nome, QTH e locatore lasciati vuoti (o riempiti dalla
 * ricerca precedente); il paese viene da cty.dat. Quello scritto a mano non si tocca.
 */
export function SstvLogCard({ dialHz }: { dialHz: number }) {
  const t = useT();
  const [f, setF] = useState<Form>(EMPTY);
  const [busy, setBusy] = useState(false);
  const [info, setInfo] = useState<{ call: string; country: string; source: string | null; state: 'found' | 'not_found' | 'off' | 'error' } | null>(null);
  // quello che ha scritto la ricerca (o l'FSK ID): si puo' sostituire; il resto e' dell'operatore
  const auto = useRef<Partial<Form>>({});
  const form = useRef(f);
  form.current = f;
  const set = (k: keyof Form, v: string) => {
    if (auto.current[k] !== undefined && auto.current[k] !== v) delete auto.current[k];
    setF((p) => ({ ...p, [k]: v }));
  };

  // il nominativo letto dall'FSK ID va nel campo, se e' vuoto o se ci aveva gia' scritto un FSK ID di prima
  useEffect(
    () =>
      onSstv((ev) => {
        if ((ev.kind !== 'rx_fskid' && ev.kind !== 'rx_done') || !ev.call) return;
        const now = form.current.call;
        if (now !== '' && auto.current.call !== now) return;
        const call = ev.call.toUpperCase();
        auto.current.call = call;
        setF((p) => ({ ...p, call }));
      }),
    [],
  );

  // la ricerca dei dati della stazione
  useEffect(() => {
    const call = f.call.trim().toUpperCase();
    if (!lookupWorthy(call)) {
      setInfo(null);
      return;
    }
    let gone = false;
    const id = window.setTimeout(() => {
      void getCallsign(call)
        .then((res) => {
          if (gone) return;
          setInfo({ call, country: res.dxcc?.country ?? res.person?.country ?? '', source: res.person?.source ?? null, state: res.lookup });
          const found = lookedUp(res);
          setF((p) => {
            if (p.call.trim().toUpperCase() !== call) return p;
            const next = { ...p };
            for (const k of ['name', 'qth', 'grid'] as const) {
              const mine = p[k] !== '' && auto.current[k] !== p[k];
              if (!mine && found[k] !== '') {
                next[k] = found[k];
                auto.current[k] = found[k];
              }
            }
            return next;
          });
        })
        .catch(() => {
          if (!gone) setInfo(null);
        });
    }, LOOKUP_DELAY_MS);
    return () => {
      gone = true;
      window.clearTimeout(id);
    };
  }, [f.call]);

  const clear = () => {
    auto.current = {};
    setInfo(null);
    setF(EMPTY);
  };

  const save = async () => {
    const call = f.call.trim().toUpperCase();
    if (!call) {
      toast(t('sstv.log.needCall'), 'error');
      return;
    }
    const now = new Date();
    setBusy(true);
    try {
      await addLogQso({
        call,
        date: now.toISOString().slice(0, 10),
        time: now.toISOString().slice(11, 16),
        band: bandOf(dialHz) ?? '',
        freq_hz: dialHz,
        mode: 'SSTV',
        rst_sent: f.sent.trim(),
        rst_rcvd: f.rcvd.trim(),
        grid: f.grid.trim(),
        name: f.name.trim(),
        qth: f.qth.trim(),
        comment: f.note.trim(),
      });
      toast(t('sstv.log.saved', { call }), 'ok');
      clear();
    } catch (error) {
      toast(replyErrorMessage(error, t), 'error');
    } finally {
      setBusy(false);
    }
  };

  const status =
    info && info.call === f.call.trim().toUpperCase()
      ? [info.country, info.state === 'found' && info.source ? t('sstv.log.from', { source: info.source }) : info.state === 'not_found' ? t('sstv.log.notFound') : ''].filter(Boolean).join(' · ')
      : '';

  return (
    <section className="sstv-log" aria-label={t('sstv.log.title')}>
      <div className="sstv-log-title">{t('sstv.log.title')}</div>
      <label className="sstv-log-f sstv-log-call">
        <span>{t('sstv.log.call')}</span>
        <input value={f.call} onChange={(e) => set('call', e.target.value.toUpperCase())} autoCapitalize="characters" spellCheck={false} />
      </label>
      {status ? (
        <p className="sstv-log-info" role="status">
          {status}
        </p>
      ) : null}
      <label className="sstv-log-f">
        <span>{t('sstv.log.his')}</span>
        <input value={f.sent} onChange={(e) => set('sent', e.target.value)} inputMode="numeric" maxLength={3} />
      </label>
      <label className="sstv-log-f">
        <span>{t('sstv.log.my')}</span>
        <input value={f.rcvd} onChange={(e) => set('rcvd', e.target.value)} inputMode="numeric" maxLength={3} />
      </label>
      <label className="sstv-log-f">
        <span>{t('sstv.log.name')}</span>
        <input value={f.name} onChange={(e) => set('name', e.target.value)} />
      </label>
      <label className="sstv-log-f">
        <span>{t('sstv.log.qth')}</span>
        <input value={f.qth} onChange={(e) => set('qth', e.target.value)} />
      </label>
      <label className="sstv-log-f">
        <span>{t('sstv.log.grid')}</span>
        <input value={f.grid} onChange={(e) => set('grid', e.target.value)} spellCheck={false} maxLength={8} />
      </label>
      <label className="sstv-log-f sstv-log-note">
        <span>{t('sstv.log.note')}</span>
        <input value={f.note} onChange={(e) => set('note', e.target.value)} />
      </label>
      <div className="sstv-log-btns">
        <button type="button" className="sstv-btn sstv-btn--primary" disabled={busy} onClick={() => void save()}>
          {t('sstv.log.save')}
        </button>
        <button type="button" className="sstv-btn" onClick={clear}>
          {t('sstv.log.clear')}
        </button>
      </div>
    </section>
  );
}
